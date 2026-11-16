//! CPS transform for [`ExprContinue`].

use crate::{
    Result,
    cps::CpsCtx,
    cps::context::CpsControlFlowTarget,
    cps::expr::CpsBind,
    cps::{CpsContinuation, CpsStep},
    error::CpsError,
    error::assert_no_attrs,
    ir::{CpsSignal, CpsTerminator, CpsTerminatorSignal},
};
use syn::ExprContinue;
use syn::Ident;

impl CpsStep for ExprContinue {
    fn then(self, _continuation: CpsContinuation, ctx: &mut CpsCtx) -> Result<CpsContinuation> {
        lower_continue(self, ctx)
    }
}

impl CpsBind for ExprContinue {
    fn and_then(
        self,
        _bind: Ident,
        _continuation: CpsContinuation,
        ctx: &mut CpsCtx,
    ) -> Result<CpsContinuation> {
        lower_continue(self, ctx)
    }
}

fn lower_continue(mut expr_continue: ExprContinue, ctx: &mut CpsCtx) -> Result<CpsContinuation> {
    assert_no_attrs(expr_continue.attrs)?;
    expr_continue.attrs = Vec::new();

    let Some(mut resolved) = ctx.resolve_control_flow(expr_continue.label.clone()) else {
        return Err(CpsError::InvalidContinue(expr_continue).into());
    };

    let CpsControlFlowTarget::Loop(target) = resolved.target() else {
        return Err(CpsError::InvalidContinue(expr_continue).into());
    };

    let target_id = target.loop_id();
    let current = resolved.current_loop().expect("at least a loop exists");
    let from = current.loop_id();

    // we could have resolve check this with idx
    let signal = if target_id == current.loop_id() {
        CpsSignal::ContinueSelf
    } else {
        current.log_targets_loop(target_id);
        resolved
            .target_child_loop()
            .expect("two loops exist")
            .log_continues_parent();

        CpsSignal::ContinueOuter {
            target: target_id.depth(),
        }
    };

    Ok(CpsContinuation::from(CpsTerminator::Signal(
        CpsTerminatorSignal { from, kind: signal },
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn continue_outside_loop_errors() {
        let expr: ExprContinue = parse_quote!(continue);
        let mut ctx = CpsCtx::default();
        let err = expr.then(CpsContinuation::empty(), &mut ctx).unwrap_err();
        assert!(matches!(*err, CpsError::InvalidContinue(..)));
    }

    #[test]
    fn continue_with_unresolved_label_errors() {
        let expr: ExprContinue = parse_quote!(continue 'missing);
        let mut ctx = CpsCtx::default();
        ctx.register_loop(None);
        assert!(matches!(
            *expr.then(CpsContinuation::empty(), &mut ctx).unwrap_err(),
            CpsError::InvalidContinue(..)
        ));
    }

    #[test]
    fn continue_with_attrs_rejected() {
        let mut expr: ExprContinue = parse_quote!(continue);
        expr.attrs = vec![parse_quote!(#[inline])];
        let mut ctx = CpsCtx::default();
        ctx.register_loop(None);
        let err = expr.then(CpsContinuation::empty(), &mut ctx).unwrap_err();
        assert!(matches!(*err, CpsError::UnallowedAttributes(..)));
    }
}
