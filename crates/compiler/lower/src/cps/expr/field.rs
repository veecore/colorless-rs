use crate::Result;
use crate::cps::anf::{HoistTarget, Hoister};
use crate::cps::expr::{CpsBind, bind_expr_and_prepend_stmt, prepend_expr_stmt};
use crate::cps::{CpsContinuation, CpsCtx, CpsStep};
use crate::ir::{CpsExpr, CpsExprField, CpsInert, CpsStmt, CpsStmtResolve};
use proc_macro2::Ident;
use syn::{Expr, ExprField, Member};

impl CpsStep for ExprField {
    fn then(self, continuation: CpsContinuation, ctx: &mut CpsCtx) -> Result<CpsContinuation> {
        lower_field(None, self, ctx, continuation)
    }
}

impl CpsBind for ExprField {
    fn and_then(
        self,
        bind: Ident,
        continuation: CpsContinuation,
        ctx: &mut CpsCtx,
    ) -> Result<CpsContinuation> {
        lower_field(Some(bind), self, ctx, continuation)
    }
}

#[inline]
fn lower_field(
    bind: Option<Ident>,
    f: ExprField,
    ctx: &mut CpsCtx,
    continuation: CpsContinuation,
) -> Result<CpsContinuation> {
    f.base.with_hoisting(
        |base, _ctx| match f.member {
            Member::Named(ident) if ident == "resolve" => {
                let resolve = CpsStmtResolve {
                    bind,
                    base,
                    dot_token: f.dot_token,
                    resolve_token: ident,
                    semi_token: (!continuation.is_empty()).then(Default::default),
                };
                Ok(CpsStmt::Resolve(resolve) + continuation)
            }
            member => {
                let field = CpsExprField {
                    base,
                    dot_token: f.dot_token,
                    member,
                };
                match bind {
                    None => Ok(prepend_expr_stmt(field, continuation)),
                    Some(bind) => Ok(bind_expr_and_prepend_stmt(bind, field, continuation)),
                }
            }
        },
        ctx,
    )
}

impl HoistTarget<CpsExpr> for ExprField {
    #[inline]
    fn hoist_into(self, hoister: &mut Hoister, ctx: &mut CpsCtx) -> Result<CpsExpr> {
        if matches!(&self.member, Member::Named(ident) if ident == "resolve") {
            Ok(hoister.hoist_as_expr(Expr::Field(self), ctx))
        } else {
            let base = self.base.hoist_into(hoister, ctx)?;
            Ok(CpsExpr::Field(CpsExprField {
                base,
                dot_token: self.dot_token,
                member: self.member,
            }))
        }
    }
}
impl HoistTarget<CpsInert> for ExprField {
    #[inline]
    fn hoist_into(self, hoister: &mut Hoister, ctx: &mut CpsCtx) -> Result<CpsInert> {
        Ok(hoister.hoist_as_inert(Expr::Field(self), ctx))
    }
}
