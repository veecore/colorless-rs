//! CPS transform for [`ExprTry`] (the `?` operator).
//!
//! `?` is frontend sugar, not a persistent CPS terminator. It is lowered to
//! the same `match` and `return` constructs used by handwritten Rust, then the
//! ordinary CPS passes distribute continuations and route returns through
//! enclosing loops.

use crate::Result;
use crate::cps::expr::CpsBind;
use crate::cps::expr::loops::path_member;
use crate::cps::{CpsContinuation, CpsCtx, CpsStep};
use crate::error::assert_no_attrs;
use crate::util::cps::{control_flow_break_pat, control_flow_continue_pat};
use crate::util::syn::{ident_syn_expr, make_syn_arm, make_syn_call};
use syn::{Expr, ExprMatch, ExprReturn, ExprTry, Ident};

impl CpsStep for ExprTry {
    fn then(self, continuation: CpsContinuation, ctx: &mut CpsCtx) -> Result<CpsContinuation> {
        desugar_try(self, ctx)?.then(continuation, ctx)
    }
}

impl CpsBind for ExprTry {
    fn and_then(
        self,
        bind: Ident,
        continuation: CpsContinuation,
        ctx: &mut CpsCtx,
    ) -> Result<CpsContinuation> {
        desugar_try(self, ctx)?.and_then(bind, continuation, ctx)
    }
}

fn desugar_try(try_expr: ExprTry, ctx: &mut CpsCtx) -> Result<ExprMatch> {
    assert_no_attrs(try_expr.attrs)?;

    let residual = ctx.next_temp_ident("residual");
    let output = ctx.next_temp_ident("val");

    let branch = make_syn_call(
        path_member(&ctx.config.try_path, "branch"),
        [*try_expr.expr],
    );
    let success = make_syn_arm(
        control_flow_continue_pat(output.clone()),
        ident_syn_expr(output),
    );

    let residual_value = make_syn_call(
        path_member(&ctx.config.try_path, "from_residual"),
        [ident_syn_expr(residual.clone())],
    );
    let failure = make_syn_arm(
        control_flow_break_pat(residual),
        Expr::Return(ExprReturn {
            attrs: Vec::new(),
            return_token: Default::default(),
            expr: Some(Box::new(residual_value)),
        }),
    );

    Ok(ExprMatch {
        attrs: Vec::new(),
        match_token: Default::default(),
        expr: Box::new(branch),
        brace_token: Default::default(),
        arms: vec![success, failure],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::scoped::{CpsScoped, CpsTerminatorScoped};
    use crate::ir::{CpsTerminator, IntoSyn};
    use quote::quote;
    use syn::parse_quote;

    #[test]
    fn try_operator_lowers_to_the_ordinary_match_terminator() {
        let expr: ExprTry = parse_quote!(step()?);
        let mut ctx = CpsCtx::default();
        let lowered = expr
            .then(CpsContinuation::empty(), &mut ctx)
            .expect("try lowering should succeed");
        let body = crate::ir::CpsBody::from(lowered);

        assert!(matches!(
            body.terminator.as_deref(),
            Some(CpsTerminator::Scoped(CpsTerminatorScoped {
                scoped: CpsScoped::Match(_),
                ..
            }))
        ));

        let restored = body.into_syn();
        let tokens = quote!(#(#restored)*).to_string();
        assert!(tokens.contains("cps_try :: branch"), "{tokens}");
        assert!(tokens.contains("cps_try :: from_residual"), "{tokens}");
    }
}
