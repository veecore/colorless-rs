use crate::{
    CpsError, Result,
    cps::CpsCtx,
    cps::anf::{HoistTarget, Hoister},
    derive_hoist_target, derive_hoist_target_placeholder,
    ir::{CpsExpr, CpsExprIndex, CpsExprLet, CpsExprRawAddr, CpsExprUnary},
    ir::{CpsFieldValue, CpsInert},
    match_expr,
};
use syn::{Expr, FieldValue};
use syn::{
    ExprAssign, ExprAwait, ExprBinary, ExprBlock, ExprBreak, ExprCall, ExprCast, ExprContinue,
    ExprForLoop, ExprGroup, ExprIf, ExprIndex, ExprLet, ExprLoop, ExprMacro, ExprMatch,
    ExprMethodCall, ExprParen, ExprRange, ExprRawAddr, ExprReference, ExprRepeat, ExprReturn,
    ExprStruct, ExprTry, ExprTryBlock, ExprUnary, ExprUnsafe, ExprWhile, ExprYield,
};

impl HoistTarget<CpsInert> for Expr {
    fn hoist_into(self, hoister: &mut Hoister, ctx: &mut CpsCtx) -> Result<CpsInert> {
        match_expr!(self => hoist_into(hoister, ctx))
    }
}

impl HoistTarget<CpsExpr> for Expr {
    fn hoist_into(self, hoister: &mut Hoister, ctx: &mut CpsCtx) -> Result<CpsExpr> {
        match_expr!(self => hoist_into(hoister, ctx))
    }
}

impl CpsFieldValue {
    fn identity(v: Self) -> Self {
        v
    }
}
derive_hoist_target! {
    FieldValue {
        _ attrs,
        member,
        colon_token,
        (pub expr),
    } -> {
        CpsFieldValue::identity(CpsFieldValue)
    }
}

derive_hoist_target! {
    ExprIndex {
        _ attrs,
        (pub expr),
        bracket_token,
        (pub index),
    } -> {
        CpsExpr::Index(CpsExprIndex)
    }
}
derive_hoist_target! {
    ExprLet {
        _ attrs,
        let_token,
        pat,
        eq_token,
        (pub expr),
    } -> {
        CpsExpr::Let(CpsExprLet)
    }
}
derive_hoist_target! {
    ExprRawAddr {
        _ attrs,
        and_token,
        raw,
        mutability,
        (pub expr),
    } -> {
        CpsExpr::RawAddr(CpsExprRawAddr)
    }
}
derive_hoist_target! {
    ExprUnary {
        _ attrs,
        op,
        (pub expr),
    } -> {
        CpsExpr::Unary(CpsExprUnary)
    }
}
derive_hoist_target_placeholder! {
    @all_inert
    ExprIndex,
    ExprLet,
    ExprRawAddr,
    ExprUnary,
}
derive_hoist_target_placeholder! {
    ExprAwait,
    ExprAssign,
    ExprBlock,
    ExprBreak,
    ExprBinary,
    ExprCall,
    ExprCast,
    ExprContinue,
    ExprForLoop,
    ExprGroup,
    ExprIf,
    ExprLoop,
    ExprMacro,
    ExprMatch,
    ExprMethodCall,
    ExprParen,
    ExprRange,
    ExprReference,
    ExprRepeat,
    ExprReturn,
    ExprStruct,
    ExprTry,
    ExprUnsafe,
    ExprWhile,
    ExprYield,
}

impl<T> HoistTarget<T> for proc_macro2::TokenStream {
    fn hoist_into(self, _hoister: &mut Hoister, _ctx: &mut CpsCtx) -> Result<T> {
        Err(CpsError::Verbatim(self).into())
    }
}
impl<T> HoistTarget<T> for ExprTryBlock {
    #[inline]
    fn hoist_into(self, _hoister: &mut Hoister, _ctx: &mut CpsCtx) -> Result<T> {
        Err(CpsError::TryBlock(self).into())
    }
}
