use crate::cps::anf::HoistTarget;
use crate::cps::context::CpsControlFlowTarget;
use crate::cps::{CpsBind, CpsContinuation, CpsCtx, CpsStep};
use crate::error::assert_no_attrs;
use crate::ir::{CpsSignal, CpsTerminator, CpsTerminatorSignal};
use crate::{CpsError, Result};
use proc_macro2::Ident;
use syn::ExprBreak;

impl CpsStep for ExprBreak {
    fn then(self, _continuation: CpsContinuation, ctx: &mut CpsCtx) -> Result<CpsContinuation> {
        lower_break(self, ctx)
    }
}

impl CpsBind for ExprBreak {
    fn and_then(
        self,
        _bind: Ident,
        _continuation: CpsContinuation,
        ctx: &mut CpsCtx,
    ) -> Result<CpsContinuation> {
        lower_break(self, ctx)
    }
}

fn lower_break(mut expr_break: ExprBreak, ctx: &mut CpsCtx) -> Result<CpsContinuation> {
    assert_no_attrs(expr_break.attrs)?;
    expr_break.attrs = Vec::new();

    let Some(mut resolved) = ctx.resolve_control_flow(expr_break.label.clone()) else {
        return Err(CpsError::InvalidBreak(expr_break).into());
    };

    match resolved.target() {
        CpsControlFlowTarget::Block(block) => {
            let cont = block.continuation.clone();
            if resolved.target_child_loop().is_some() {
                return Err(CpsError::InvalidBreak(expr_break).into());
            }

            expr_break.expr.maybe_then(cont, ctx)
        }
        CpsControlFlowTarget::Loop(l) => {
            l.log_breaks_self();
            let target = l.loop_id();
            let current_loop = resolved.current_loop().expect("at least a loop exists");
            current_loop.log_targets_loop(target);
            let from = current_loop.loop_id();

            expr_break.expr.map(|e| *e).with_hoisting(
                |expr, _ctx| {
                    let signal = CpsTerminator::Signal(CpsTerminatorSignal {
                        from,
                        kind: CpsSignal::BreakOuter {
                            target: target.depth(),
                            val: expr,
                        },
                    });
                    Ok(CpsContinuation::from(signal))
                },
                ctx,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn break_outside_loop_errors() {
        let expr: ExprBreak = parse_quote!(break);
        let mut ctx = CpsCtx::default();
        let err = expr.then(CpsContinuation::empty(), &mut ctx).unwrap_err();
        assert!(matches!(*err, CpsError::InvalidBreak(..)));
    }

    #[test]
    fn break_with_unresolved_label_errors() {
        let expr: ExprBreak = parse_quote!(break 'missing);
        let mut ctx = CpsCtx::default();
        ctx.register_loop(None);
        assert!(matches!(
            *expr.then(CpsContinuation::empty(), &mut ctx).unwrap_err(),
            CpsError::InvalidBreak(..)
        ));
    }

    #[test]
    fn break_with_attrs_rejected() {
        let mut expr: ExprBreak = parse_quote!(break);
        expr.attrs = vec![parse_quote!(#[inline])];
        let mut ctx = CpsCtx::default();
        ctx.register_loop(None);
        let err = expr.then(CpsContinuation::empty(), &mut ctx).unwrap_err();
        assert!(matches!(*err, CpsError::UnallowedAttributes(..)));
    }
}
