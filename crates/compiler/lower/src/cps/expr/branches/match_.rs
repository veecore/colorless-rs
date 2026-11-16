use crate::{
    CpsStep, Result,
    cps::anf::HoistTarget,
    cps::continuation::ScopeEnd,
    cps::{CpsBind, CpsContinuation, CpsCtx},
    derive_cps_bind_scope_delegate,
    error::assert_no_attrs,
    ir::scoped::{CpsArm, CpsScoped, CpsScopedMatch},
};
use proc_macro2::Ident;
use syn::{Arm, ExprMatch};

derive_cps_bind_scope_delegate!(ExprMatch => lower_match / end_branch_scope);

#[inline]
fn lower_match(m: ExprMatch, scope_end: ScopeEnd, ctx: &mut CpsCtx) -> Result<CpsContinuation> {
    assert_no_attrs(m.attrs)?;
    let cont = scope_end.continuation().cloned();
    m.expr.with_hoisting(
        |scrutinee, ctx| {
            let arms = m.arms.maybe_then(cont.clone(), ctx)?;

            Ok(scope_end.into_terminator(CpsScoped::Match(CpsScopedMatch {
                match_token: m.match_token,
                expr: scrutinee,
                brace_token: m.brace_token,
                arms,
            })))
        },
        ctx,
    )
}

impl CpsStep<Vec<CpsArm>> for Vec<Arm> {
    fn then(self, c: CpsContinuation, ctx: &mut CpsCtx) -> Result<Vec<CpsArm>> {
        self.into_iter()
            .map(|arm| arm.then(c.clone(), ctx))
            .collect()
    }
}

impl CpsBind<Vec<CpsArm>> for Vec<Arm> {
    fn and_then(self, bind: Ident, c: CpsContinuation, ctx: &mut CpsCtx) -> Result<Vec<CpsArm>> {
        self.into_iter()
            .map(|arm| arm.and_then(bind.clone(), c.clone(), ctx))
            .collect()
    }
}

impl CpsStep<CpsArm> for Arm {
    #[inline]
    fn then(self, c: CpsContinuation, ctx: &mut CpsCtx) -> Result<CpsArm> {
        let Arm {
            attrs,
            body,
            pat,
            fat_arrow_token,
            comma,
        } = self;
        assert_no_attrs(attrs)?;

        let body = CpsStep::<CpsContinuation>::then(body, c, ctx)?;
        let body = body.into();
        Ok(CpsArm {
            body,
            pat,
            fat_arrow_token,
            comma,
        })
    }
}
impl CpsBind<CpsArm> for Arm {
    #[inline]
    fn and_then(self, bind: Ident, c: CpsContinuation, ctx: &mut CpsCtx) -> Result<CpsArm> {
        let Arm {
            attrs,
            body,
            pat,
            fat_arrow_token,
            comma,
        } = self;
        assert_no_attrs(attrs)?;

        let body = CpsBind::<CpsContinuation>::and_then(body, bind, c, ctx)?;
        let body = body.into();
        Ok(CpsArm {
            body,
            pat,
            fat_arrow_token,
            comma,
        })
    }
}
