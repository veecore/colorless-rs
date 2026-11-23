use crate::ir::CpsPlainBlock;
use crate::ir_enum;
use proc_macro2::Ident;
use syn::punctuated::Punctuated;
use syn::{
    AngleBracketedGenericArguments, BinOp, Block, BlockModifiers, BoundLifetimes, ClosureModifiers,
    Expr, Lit, Macro, Member, Pat, Path, PointerMutability, QSelf, RangeLimits, ReturnType, Token,
    Type, UnOp, token,
};

ir_enum! {
    /// An expression whose evaluation invokes no user-defined runtime behavior
    /// and introduces no CPS control point.
    ///
    /// It may construct, copy, or move values. Destruction of its resulting value
    /// is controlled by the surrounding Rust expression.
    #[derive(Clone, Debug)]
    pub enum CpsInert {
        Array(CpsExprArray),
        Async(CpsExprAsync),
        Closure(CpsExprClosure),
        Const(CpsExprConst),
        Infer(CpsExprInfer),
        Lit(CpsExprLit),
        Path(CpsExprPath),
        TempHandoff(pub CpsTempHandoff),
        Tuple(CpsExprTuple),
    }
}

#[derive(Clone, Debug)]
pub struct CpsExprArray {
    pub bracket_token: token::Bracket,
    pub elems: Punctuated<CpsInert, Token![,]>,
}

#[derive(Clone, Debug)]
pub struct CpsExprAsync {
    pub async_token: Token![async],
    pub capture: Option<Token![move]>,
    pub modifiers: BlockModifiers,
    pub block: Block,
}

#[derive(Clone, Debug)]
pub struct CpsExprClosure {
    pub lifetimes: Option<BoundLifetimes>,
    pub modifiers: ClosureModifiers,
    pub constness: Option<Token![const]>,
    pub asyncness: Option<Token![async]>,
    pub capture: Option<Token![move]>,
    pub inputs_begin: Token![|],
    pub inputs: Punctuated<Pat, Token![,]>,
    pub inputs_end: Token![|],
    pub output: ReturnType,
    pub body: Box<Expr>,
}

#[derive(Clone, Debug)]
pub struct CpsExprConst {
    pub const_token: Token![const],
    pub modifiers: BlockModifiers,
    pub block: Block,
}

#[derive(Clone, Debug)]
pub struct CpsExprInfer {
    pub underscore_token: Token![_],
}

#[derive(Clone, Debug)]
pub struct CpsExprLit {
    pub lit: Lit,
}

#[derive(Clone, Debug)]
pub struct CpsExprPath {
    pub qself: Option<QSelf>,
    pub path: Path,
}

/// Transfers ownership of a hoisted temporary back to its original use site.
///
/// Emits:
///
/// ```ignore
/// {
///     let tmp = tmp;
///     tmp
/// }
/// ```
///
/// The outer binding provides ordered ANF storage. The shadowing binding
/// consumes that storage and presents the value to Rust as a temporary at the
/// reconstructed operand position, restoring Rust's ordinary temporary and
/// drop-scope rules.
#[derive(Clone, Debug)]
pub struct CpsTempHandoff {
    pub ident: Ident,
}

#[derive(Clone, Debug)]
pub struct CpsExprTuple {
    pub paren_token: token::Paren,
    pub elems: Punctuated<CpsInert, Token![,]>,
}

// ======================== Expr ========================

ir_enum! {
    #[derive(Clone, Debug)]
    pub enum CpsExpr {
        Assign(CpsExprAssign),
        Await(CpsExprAwait),
        Binary(CpsExprBinary),
        Block(CpsExprBlock),
        Call(CpsExprCall),
        Cast(CpsExprCast),
        Field(CpsExprField),
        Group(CpsExprGroup),
        Inert(pub CpsInert),
        Index(CpsExprIndex),
        Let(CpsExprLet),
        MethodCall(CpsExprMethodCall),
        Macro(CpsExprMacro),
        Paren(CpsExprParen),
        Range(CpsExprRange),
        RawAddr(CpsExprRawAddr),
        Reference(CpsExprReference),
        Repeat(CpsExprRepeat),
        Struct(CpsExprStruct),
        Unary(CpsExprUnary),
        Yield(CpsExprYield),
    }
}

#[derive(Clone, Debug)]
pub struct CpsExprAssign {
    pub left: Box<CpsExpr>,
    pub eq_token: Token![=],
    pub right: Box<CpsInert>,
}

#[derive(Clone, Debug)]
pub struct CpsExprAwait {
    pub base: Box<CpsExpr>,
    pub dot_token: Token![.],
    pub await_token: Token![await],
}

#[derive(Clone, Debug)]
pub struct CpsExprBinary {
    pub left: Box<CpsExpr>,
    pub op: BinOp,
    pub right: Box<CpsInert>,
}

#[derive(Clone, Debug)]
pub struct CpsExprBlock {
    pub block: CpsPlainBlock,
}

#[derive(Clone, Debug)]
pub struct CpsExprCall {
    pub func: Box<CpsInert>,
    pub paren_token: token::Paren,
    pub args: Punctuated<CpsInert, Token![,]>,
}

#[derive(Clone, Debug)]
pub struct CpsExprCast {
    pub expr: Box<CpsExpr>,
    pub as_token: Token![as],
    pub ty: Box<Type>,
}

#[derive(Clone, Debug)]
pub struct CpsExprField {
    pub base: Box<CpsExpr>,
    pub dot_token: Token![.],
    pub member: Member,
}

#[derive(Clone, Debug)]
pub struct CpsExprGroup {
    pub group_token: token::Group,
    pub expr: Box<CpsExpr>,
}

// Base could be inert and not sometimes... in
// *self.a[1] = 1... it can't be hoisted... also,
// rust runs 1 first, so it seems okay
#[derive(Clone, Debug)]
pub struct CpsExprIndex {
    pub expr: Box<CpsExpr>,
    pub bracket_token: token::Bracket,
    pub index: Box<CpsInert>,
}

#[derive(Clone, Debug)]
pub struct CpsExprLet {
    pub let_token: Token![let],
    pub pat: Box<Pat>,
    pub eq_token: Token![=],
    pub expr: Box<CpsExpr>,
}

#[derive(Clone, Debug)]
pub struct CpsExprMethodCall {
    // Deref makes this an issue
    pub receiver: Box<CpsExpr>,
    pub dot_token: Token![.],
    pub method: Ident,
    pub turbofish: Option<AngleBracketedGenericArguments>,
    pub paren_token: token::Paren,
    pub args: Punctuated<CpsInert, Token![,]>,
}

#[derive(Clone, Debug)]
pub struct CpsExprMacro {
    pub mac: Macro,
}

#[derive(Clone, Debug)]
pub struct CpsExprParen {
    pub paren_token: token::Paren,
    pub expr: Box<CpsExpr>,
}

#[derive(Clone, Debug)]
pub struct CpsExprRange {
    pub start: Option<Box<CpsInert>>,
    pub limits: RangeLimits,
    pub end: Option<Box<CpsInert>>,
}

#[derive(Clone, Debug)]
pub struct CpsExprRawAddr {
    pub and_token: Token![&],
    pub raw: Token![raw],
    pub mutability: PointerMutability,
    pub expr: Box<CpsExpr>,
}

#[derive(Clone, Debug)]
pub struct CpsExprReference {
    pub and_token: Token![&],
    pub mutability: Option<Token![mut]>,
    pub expr: Box<CpsExpr>,
}

#[derive(Clone, Debug)]
pub struct CpsExprRepeat {
    pub bracket_token: token::Bracket,
    pub expr: Box<CpsInert>,
    pub semi_token: Token![;],
    pub len: Box<CpsInert>,
}

#[derive(Clone, Debug)]
pub struct CpsExprStruct {
    pub qself: Option<QSelf>,
    pub path: Path,
    pub brace_token: token::Brace,
    pub fields: Punctuated<CpsFieldValue, Token![,]>,
    pub dot2_token: Option<Token![..]>,
    pub rest: Option<Box<CpsInert>>,
}

#[derive(Clone, Debug)]
pub struct CpsExprUnary {
    pub op: UnOp,
    pub expr: Box<CpsExpr>,
}

#[derive(Clone, Debug)]
pub struct CpsExprYield {
    pub yield_token: Token![yield],
    pub expr: Option<Box<CpsExpr>>,
}

#[derive(Clone, Debug)]
pub struct CpsFieldValue {
    pub member: Member,
    pub colon_token: Option<Token![:]>,
    pub expr: CpsInert,
}

mod convert_impls_inert {
    use crate::derive_into_syn;
    use crate::ir::expr::{
        CpsExprArray, CpsExprAsync, CpsExprClosure, CpsExprConst, CpsExprInfer, CpsExprLit,
        CpsExprPath, CpsExprTuple, CpsInert,
    };
    use crate::ir::{CpsTempHandoff, IntoSyn};
    use syn::{
        Expr, ExprArray, ExprAsync, ExprClosure, ExprConst, ExprInfer, ExprLit, ExprPath,
        ExprTuple, parse_quote,
    };

    impl IntoSyn<Expr> for CpsInert {
        fn into_syn(self) -> Expr {
            CpsInert!(self => Expr => into_syn())
        }
    }

    derive_into_syn! {
        ExprArray {
            _ attrs,
            bracket_token,
            (pub elems),
        } -> {
            CpsExprArray
        }
    }
    derive_into_syn! {
        ExprAsync {
            _ attrs,
            async_token,
            capture,
            modifiers,
            block,
        } <-> {
            CpsExprAsync
        }
    }
    derive_into_syn! {
        ExprClosure {
            _ attrs,
            lifetimes,
            modifiers,
            constness,
            asyncness,
            capture,
            inputs_begin,
            inputs,
            inputs_end,
            output,
            body,
        } <-> {
            CpsExprClosure
        }
    }
    derive_into_syn! {
        ExprConst {
            _ attrs,
            const_token,
            modifiers,
            block,
        } <-> {
            CpsExprConst
        }
    }
    derive_into_syn! {
        ExprInfer {
            _ attrs,
            underscore_token,
        } <-> {
            CpsExprInfer
        }
    }
    derive_into_syn! {
        ExprLit {
            _ attrs,
            lit,
        } <-> {
            CpsExprLit
        }
    }
    derive_into_syn! {
        ExprPath {
            _ attrs,
            qself,
            path,
        } <-> {
            CpsExprPath
        }
    }
    derive_into_syn! {
        ExprTuple {
            _ attrs,
            paren_token,
            (pub elems),
        } -> {
            CpsExprTuple
        }
    }
    impl IntoSyn<Expr> for CpsTempHandoff {
        fn into_syn(self) -> Expr {
            let tmp = self.ident;
            parse_quote!({
                let #tmp = #tmp;
                #tmp
            })
        }
    }
    macro_rules! derive_into_expr {
        ($($ty:ident),*) => {
            $(
                impl From<$ty> for $crate::ir::CpsExpr {
                    #[inline]
                    fn from(v: $ty) -> Self {
                        let inert = $crate::ir::CpsInert::from(v);
                        $crate::ir::CpsExpr::Inert(inert)
                    }
                }
            )*
        };
    }

    derive_into_expr! {
        CpsExprArray,
        CpsExprAsync,
        CpsExprClosure,
        CpsExprConst,
        CpsExprInfer,
        CpsExprLit,
        CpsExprPath,
        CpsExprTuple
    }
}

mod convert_impls {
    use crate::derive_into_syn;
    use crate::ir::expr::{
        CpsExpr, CpsExprAssign, CpsExprAwait, CpsExprBinary, CpsExprCall, CpsExprCast,
        CpsExprField, CpsExprIndex, CpsExprLet, CpsExprMacro, CpsExprMethodCall, CpsExprParen,
        CpsExprRange, CpsExprRawAddr, CpsExprReference, CpsExprRepeat, CpsExprStruct, CpsExprUnary,
        CpsExprYield, CpsFieldValue,
    };
    use crate::ir::{CpsExprBlock, CpsExprGroup, IntoSyn};
    use syn::{
        Expr, ExprAssign, ExprAwait, ExprBinary, ExprBlock, ExprCall, ExprCast, ExprField,
        ExprGroup, ExprIndex, ExprLet, ExprMacro, ExprMethodCall, ExprParen, ExprRange,
        ExprRawAddr, ExprReference, ExprRepeat, ExprStruct, ExprUnary, ExprYield, FieldValue,
        Macro,
    };

    impl IntoSyn<Expr> for CpsExpr {
        fn into_syn(self) -> Expr {
            CpsExpr!(self => Expr => into_syn())
        }
    }

    derive_into_syn! {
        ExprAssign {
            _ attrs,
            (pub left),
            eq_token,
            (pub right),
        } -> {
            CpsExprAssign
        }
    }
    derive_into_syn! {
        ExprAwait {
            _ attrs,
            (pub base),
            dot_token,
            await_token,
        } -> {
            CpsExprAwait
        }
    }
    derive_into_syn! {
        ExprBinary {
            _ attrs,
            (pub left),
            op,
            (pub right),
        } -> {
            CpsExprBinary
        }
    }
    derive_into_syn! {
        ExprBlock {
            _ attrs,
            (pub block),
            _ label,
        } -> {
            CpsExprBlock
        }
    }
    derive_into_syn! {
        ExprCall {
            _ attrs,
            (pub func),
            paren_token,
            (pub args),
        } -> {
            CpsExprCall
        }
    }
    derive_into_syn! {
        ExprCast {
            _ attrs,
            (pub expr),
            as_token,
            ty,
        } -> {
            CpsExprCast
        }
    }
    derive_into_syn! {
        ExprField {
            _ attrs,
            (pub base),
            dot_token,
            member,
        } -> {
            CpsExprField
        }
    }
    derive_into_syn! {
        ExprGroup {
            _ attrs,
            (pub expr),
            group_token,
        } -> {
            CpsExprGroup
        }
    }
    derive_into_syn! {
        ExprIndex {
            _ attrs,
            (pub expr),
            bracket_token,
            (pub index),
        } -> {
            CpsExprIndex
        }
    }
    derive_into_syn! {
        ExprLet {
            _ attrs,
            let_token,
            pat,
            eq_token,
            (pub expr),
        } -> { CpsExprLet }
    }
    derive_into_syn! {
        ExprMacro {
            _ attrs,
            (pub mac),
        } -> {
            CpsExprMacro
        }
    }
    derive_into_syn! {
        ExprMethodCall {
            _ attrs,
            (pub receiver),
            dot_token,
            method,
            turbofish,
            paren_token,
            (pub args),
        } -> {
            CpsExprMethodCall
        }
    }
    derive_into_syn! {
        ExprParen {
            _ attrs,
            paren_token,
            (pub expr),
        } -> {
            CpsExprParen
        }
    }
    derive_into_syn! {
        ExprRange {
            _ attrs,
            (pub start),
            limits,
            (pub end),
        } -> {
            CpsExprRange
        }
    }
    derive_into_syn! {
        ExprRawAddr {
            _ attrs,
            and_token,
            raw,
            mutability,
            (pub expr),
        } -> {
            CpsExprRawAddr
        }
    }
    derive_into_syn! {
        ExprReference {
            _ attrs,
            and_token,
            mutability,
            (pub expr),
        } -> {
            CpsExprReference
        }
    }
    derive_into_syn! {
        ExprRepeat {
            _ attrs,
            bracket_token,
            (pub expr),
            semi_token,
            (pub len),
        } -> {
            CpsExprRepeat
        }
    }
    derive_into_syn! {
        ExprStruct {
            _ attrs,
            qself,
            path,
            brace_token,
            (pub fields),
            dot2_token,
            (pub rest),
        } -> {
            CpsExprStruct
        }
    }
    derive_into_syn! {
        ExprUnary {
            _ attrs,
            op,
            (pub expr),
        } -> {
            CpsExprUnary
        }
    }
    derive_into_syn! {
        ExprYield {
            _ attrs,
            yield_token,
            (pub expr),
        } -> {
            CpsExprYield
        }
    }

    derive_into_syn! {
        FieldValue {
            _ attrs,
            member,
            colon_token,
            (pub expr),
        } -> {
            CpsFieldValue
        }
    }

    impl IntoSyn<Macro> for Macro {
        #[inline]
        fn into_syn(self) -> Macro {
            self
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::ops::{Deref, DerefMut};

    #[test]
    fn assignment_evaluates_the_right_before_the_place() {
        struct Place<'a> {
            order: &'a RefCell<Vec<&'static str>>,
            value: i32,
        }

        impl Place<'_> {
            fn get(&mut self) -> &mut i32 {
                self.order.borrow_mut().push("left");
                &mut self.value
            }
        }

        let order = RefCell::new(Vec::new());
        let mut place = Place {
            order: &order,
            value: 0,
        };
        *place.get() = {
            order.borrow_mut().push("right");
            1
        };

        assert_eq!(*order.borrow(), ["right", "left"]);
    }

    #[test]
    fn binary_evaluates_left_to_right() {
        let order = RefCell::new(Vec::new());
        let _ = {
            order.borrow_mut().push("left");
            1
        } + {
            order.borrow_mut().push("right");
            2
        };

        assert_eq!(*order.borrow(), ["left", "right"]);
    }

    #[test]
    fn compound_assignment_evaluates_the_right_before_mutably_dereferencing_the_place() {
        struct Parent<'a> {
            b: Child,
            order: &'a RefCell<Vec<&'static str>>,
        }

        struct Child {
            c: i32,
        }

        impl Deref for Parent<'_> {
            type Target = Child;

            fn deref(&self) -> &Self::Target {
                &self.b
            }
        }

        impl DerefMut for Parent<'_> {
            fn deref_mut(&mut self) -> &mut Self::Target {
                self.order.borrow_mut().push("left");
                &mut self.b
            }
        }

        let order = RefCell::new(Vec::new());
        let mut parent = Parent {
            b: Child { c: 0 },
            order: &order,
        };
        parent.c += {
            order.borrow_mut().push("right");
            1
        };

        assert_eq!(*order.borrow(), ["right", "left"]);
    }

    #[test]
    fn method_call_adjusts_the_receiver_before_evaluating_arguments() {
        struct TestMethodCall<'a> {
            order: &'a RefCell<Vec<&'static str>>,
        }

        impl Deref for TestMethodCall<'_> {
            type Target = str;

            fn deref(&self) -> &Self::Target {
                self.order.borrow_mut().push("base");
                "value"
            }
        }

        let order = RefCell::new(Vec::new());
        let value = TestMethodCall { order: &order };
        value.matches({
            order.borrow_mut().push("arg");
            'v'
        });

        assert_eq!(*order.borrow(), ["base", "arg"]);
    }

    #[test]
    fn function_call_evaluates_the_callee_before_arguments() {
        let order = RefCell::new(Vec::new());
        let callee = || {
            order.borrow_mut().push("func");
            (|_: i32| {}) as fn(i32)
        };
        callee()({
            order.borrow_mut().push("arg");
            1
        });

        assert_eq!(*order.borrow(), ["func", "arg"]);
    }

    #[test]
    fn paren() {
        // Ok that this compiles means we couldn't
        // make the inner expression inert
        *(&mut 1) = 1;
    }
}
