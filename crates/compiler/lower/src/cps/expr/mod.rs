//! # Expression Lowering Pipeline
//!
//! Categorized expression transformations:
//! - [`leaf`]: Atomic leaf expressions that never diverge (Literals, Paths, Idents).
//! - [`compound`]: Pure ANF compound expressions with hoisted sub-expressions (Calls, Arrays, Tuples...).
//! - [`branches`]: Control-flow branching (`ExprIf`, `ExprMatch`).
//! - [`loops`]: Structured loop lowerings (`loop`, `while`, `for`) and signal pattern generation.
//! - [`jumps`]: Non-local control flow exits (`break`, `continue`, `return`, `try ?`).
//! - [`blocks`]: Labeled, plain, and unsafe blocks.

pub mod binary;
pub mod blocks;
pub mod branches;
pub mod compound;
pub mod field;
pub mod jumps;
pub mod leaf;
pub mod loops;
pub mod macro_;

use crate::cps::context::CpsCtx;
use crate::cps::continuation::CpsContinuation;
use crate::cps::{CpsBind, CpsStep};
use crate::ir::{CpsBlock, CpsExpr};
use crate::util::cps::make_bind;
use crate::util::syn::unit_syn_expr;
use crate::{CpsError, Result, match_expr};
use proc_macro2::Ident;
use syn::Expr;
use syn::token::Else;

impl CpsStep for Expr {
    fn then(self, c: CpsContinuation, ctx: &mut CpsCtx) -> Result<CpsContinuation> {
        match_expr!(self => then (c, ctx))
    }
}

impl CpsBind for Expr {
    fn and_then(
        self,
        bind: Ident,
        continuation: CpsContinuation,
        ctx: &mut CpsCtx,
    ) -> Result<CpsContinuation> {
        match_expr!(self => and_then (bind, continuation, ctx))
    }
}

/// CPS transform for an expression that is *always* diverging (used by
/// `let`-`else` desugaring to lower the escape branch).
///
/// Because this crate operates on AST syntax without type information,
/// `converge` lowers the `else` block without verifying that it truly diverges.
/// Non-diverging code such as `else { 2 }` will compile to a reachable arm.
/// Callers that need soundness (e.g. `#[lower_cps]`) emit a copy of the
/// original function in a `const _: () = { ... }` block so `rustc` enforces
/// divergence on the source AST.
pub trait DivergingExpr {
    fn converge(self, ctx: &mut CpsCtx) -> Result<CpsBlock>;
}

impl DivergingExpr for (Else, Box<Expr>) {
    fn converge(self, ctx: &mut CpsCtx) -> Result<CpsBlock> {
        let buf: CpsContinuation = self.1.step_with(&mut *ctx)?;

        if !ctx.config.allow_maybe_diverging_in_let_else {
            return Err(CpsError::UnclearDivergence(self.0).into());
        }

        Ok(buf.into())
    }
}

impl CpsBind for Option<Box<Expr>> {
    fn and_then(
        self,
        bind: Ident,
        continuation: CpsContinuation,
        ctx: &mut CpsCtx,
    ) -> Result<CpsContinuation> {
        match self {
            Some(val) => (*val).and_then(bind, continuation, ctx),
            None => unit_syn_expr().and_then(bind, continuation, ctx),
        }
    }
}

#[inline(always)]
pub fn bind_expr_and_prepend_stmt(
    bind: Ident,
    value: impl Into<CpsExpr>,
    cont: CpsContinuation,
) -> CpsContinuation {
    let value: CpsExpr = value.into();
    let app_stmt = make_bind(bind, value);
    app_stmt + cont
}

#[inline(always)]
pub fn prepend_expr_stmt(value: impl Into<CpsExpr>, cont: CpsContinuation) -> CpsContinuation {
    let value: CpsExpr = value.into();
    value + cont
}

mod macros {
    #[macro_export]
    macro_rules! match_expr {
        ($expr:expr => $method:ident ($($args:tt)*)) => {
            match $expr {
                Expr::Array(array) => array.$method($($args)*),
                Expr::Assign(assign) => assign.$method($($args)*),
                Expr::Async(async_e) => async_e.$method($($args)*),
                Expr::Await(await_e) => await_e.$method($($args)*),
                Expr::Binary(binary) => binary.$method($($args)*),
                Expr::Block(block) => block.$method($($args)*),
                Expr::Break(break_e) => break_e.$method($($args)*),
                Expr::Call(call) => call.$method($($args)*),
                Expr::Cast(cast) => cast.$method($($args)*),
                Expr::Closure(closure) => closure.$method($($args)*),
                Expr::Const(const_e) => const_e.$method($($args)*),
                Expr::Continue(continue_e) => continue_e.$method($($args)*),
                Expr::Field(field_e) => field_e.$method($($args)*),
                Expr::ForLoop(for_loop) => for_loop.$method($($args)*),
                Expr::Group(group) => group.$method($($args)*),
                Expr::If(if_e) => if_e.$method($($args)*),
                Expr::Index(index) => index.$method($($args)*),
                Expr::Infer(infer) => infer.$method($($args)*),
                Expr::Let(let_e) => let_e.$method($($args)*),
                Expr::Lit(lit) => lit.$method($($args)*),
                Expr::Loop(loop_e) => loop_e.$method($($args)*),
                Expr::Macro(macro_e) => macro_e.$method($($args)*),
                Expr::Match(match_e) => match_e.$method($($args)*),
                Expr::MethodCall(method_call) => {
                    method_call.$method($($args)*)
                }
                Expr::Paren(paren) => paren.$method($($args)*),
                Expr::Path(path) => path.$method($($args)*),
                Expr::Range(range) => range.$method($($args)*),
                Expr::RawAddr(raw_addr) => raw_addr.$method($($args)*),
                Expr::Reference(reference) => reference.$method($($args)*),
                Expr::Repeat(repeat) => repeat.$method($($args)*),
                Expr::Return(return_e) => return_e.$method($($args)*),
                Expr::Struct(struct_e) => struct_e.$method($($args)*),
                Expr::Try(try_e) => try_e.$method($($args)*),
                Expr::TryBlock(try_block) => try_block.$method($($args)*),
                Expr::Tuple(tuple) => tuple.$method($($args)*),
                Expr::Unary(unary) => unary.$method($($args)*),
                Expr::Unsafe(unsafe_e) => unsafe_e.$method($($args)*),
                Expr::Verbatim(verbatim) => verbatim.$method($($args)*),
                Expr::While(while_e) => while_e.$method($($args)*),
                Expr::Yield(yield_e) => yield_e.$method($($args)*),
                _ => unimplemented!(),
            }
        };
        (ExprArray) => {
            syn::Expr::Array
        };
        (ExprAssign) => {
            syn::Expr::Assign
        };
        (ExprAsync) => {
            syn::Expr::Async
        };
        (ExprAwait) => {
            syn::Expr::Await
        };
        (ExprBinary) => {
            syn::Expr::Binary
        };
        (ExprBlock) => {
            syn::Expr::Block
        };
        (ExprBreak) => {
            syn::Expr::Break
        };
        (ExprCall) => {
            syn::Expr::Call
        };
        (ExprCast) => {
            syn::Expr::Cast
        };
        (ExprConst) => {
            syn::Expr::Const
        };
        (ExprContinue) => {
            syn::Expr::Continue
        };
        (ExprClosure) => {
            syn::Expr::Closure
        };
        (ExprField) => {
            syn::Expr::Field
        };
        (ExprForLoop) => {
            syn::Expr::ForLoop
        };
        (ExprGroup) => {
            syn::Expr::Group
        };
        (ExprIf) => {
            syn::Expr::If
        };
        (ExprIndex) => {
            syn::Expr::Index
        };
        (ExprInfer) => {
            syn::Expr::Infer
        };
        (ExprLet) => {
            syn::Expr::Let
        };
        (ExprLit) => {
            syn::Expr::Lit
        };
        (ExprLoop) => {
            syn::Expr::Loop
        };
        (ExprMacro) => {
            syn::Expr::Macro
        };
        (ExprMatch) => {
            syn::Expr::Match
        };
        (ExprMethodCall) => {
            syn::Expr::MethodCall
        };
        (ExprParen) => {
            syn::Expr::Paren
        };
        (ExprPath) => {
            syn::Expr::Path
        };
        (ExprRange) => {
            syn::Expr::Range
        };
        (ExprRawAddr) => {
            syn::Expr::RawAddr
        };
        (ExprReference) => {
            syn::Expr::Reference
        };
        (ExprReturn) => {
            syn::Expr::Return
        };
        (ExprRepeat) => {
            syn::Expr::Repeat
        };
        (ExprStruct) => {
            syn::Expr::Struct
        };
        (ExprTuple) => {
            syn::Expr::Tuple
        };
        (ExprTry) => {
            syn::Expr::Try
        };
        (ExprTryBlock) => {
            syn::Expr::TryBlock
        };
        (ExprUnary) => {
            syn::Expr::Unary
        };
        (ExprUnsafe) => {
            syn::Expr::Unsafe
        };
        (ExprWhile) => {
            syn::Expr::While
        };
        (ExprYield) => {
            syn::Expr::Yield
        };
        ($f:ident) => {
            compile_error!(concat!("unsupported expression constructor: ", stringify!($f)))
        }
    }
}
