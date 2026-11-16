use crate::cps::anf::HoistTarget;
use crate::cps::continuation::ScopeEnd;
use crate::cps::{CpsBind, CpsContinuation, CpsCtx, CpsStep};
use crate::error::assert_no_attrs;
use crate::ir::scoped::{CpsElse, CpsScoped, CpsScopedIf};
use crate::ir::{CpsBlock, CpsExpr};
use crate::{CpsError, Result, derive_cps_bind_scope_delegate};
use proc_macro2::Ident;
use syn::token::{Else, If};
use syn::{Expr, ExprIf};

derive_cps_bind_scope_delegate!(ExprIf => lower_if_ / end_branch_scope);

#[inline]
fn lower_if_(m: ExprIf, scope_end: ScopeEnd, ctx: &mut CpsCtx) -> Result<CpsContinuation> {
    assert_no_attrs(m.attrs)?;
    let cont = scope_end.continuation().cloned();
    m.cond.with_hoisting_test(
        scope_end,
        m.if_token,
        m.then_branch.maybe_then(cont.clone(), ctx)?,
        (m.if_token, m.else_branch).maybe_then(cont.clone(), ctx)?,
        ctx,
    )
}

impl CpsStep<CpsElse> for (If, Option<(Else, Box<Expr>)>) {
    #[inline]
    fn then(self, c: CpsContinuation, ctx: &mut CpsCtx) -> Result<CpsElse> {
        match self.1 {
            Some((els, exp)) => {
                let expr = exp.then(c, ctx)?;
                Ok(CpsElse {
                    else_token: els,
                    block: expr.into(),
                })
            }
            None => Ok(CpsElse {
                else_token: Default::default(),
                block: c.into(),
            }),
        }
    }
}

impl CpsBind<CpsElse> for (If, Option<(Else, Box<Expr>)>) {
    #[inline]
    fn and_then(self, bind: Ident, c: CpsContinuation, ctx: &mut CpsCtx) -> Result<CpsElse> {
        match self.1 {
            Some((els, exp)) => {
                let expr = exp.and_then(bind, c, ctx)?;
                Ok(CpsElse {
                    else_token: els,
                    block: expr.into(),
                })
            }
            // TODO; NOT IF THE IF EVALUATES TO ()... F0RCE IT TO
            None => Err(CpsError::MissingElse(self.0).into()),
        }
    }
}

// could we make this return if
pub trait HoistTargetTest: HoistTarget<CpsExpr> {
    fn with_hoisting_test(
        self,
        scope_end: ScopeEnd,
        if_token: If,
        then_branch: CpsBlock,
        else_branch: CpsElse,
        ctx: &mut CpsCtx,
    ) -> Result<CpsContinuation>
    where
        Self: Sized,
    {
        default_hoist_test(self, scope_end, if_token, then_branch, else_branch, ctx)
    }
}

impl HoistTargetTest for Expr {
    fn with_hoisting_test(
        self,
        scope_end: ScopeEnd,
        if_token: If,
        then_branch: CpsBlock,
        else_branch: CpsElse,
        ctx: &mut CpsCtx,
    ) -> Result<CpsContinuation>
    where
        Self: Sized,
    {
        match self {
            Expr::Binary(bin) => {
                bin.with_hoisting_test(scope_end, if_token, then_branch, else_branch, ctx)
            }
            other => default_hoist_test(other, scope_end, if_token, then_branch, else_branch, ctx),
        }
    }
}

pub fn default_hoist_test<S>(
    self_: S,
    scope_end: ScopeEnd,
    if_token: If,
    then_branch: CpsBlock,
    else_branch: CpsElse,
    ctx: &mut CpsCtx,
) -> Result<CpsContinuation>
where
    S: HoistTarget<CpsExpr>,
{
    self_.with_hoisting(
        |scrutinee, _ctx| {
            Ok(scope_end.into_terminator(CpsScoped::If(CpsScopedIf {
                if_token,
                cond: scrutinee.into(),
                then_branch,
                else_branch,
            })))
        },
        ctx,
    )
}
