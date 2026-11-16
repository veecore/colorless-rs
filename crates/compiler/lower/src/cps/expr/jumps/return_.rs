//! CPS transform for [`ExprReturn`].
//!
//! In continuation-passing style, an early `return` diverges:
//! * The incoming continuation is dropped (dead-code elimination).
//! * The return expression itself (or `()` if omitted) is evaluated with an empty continuation.
//! * Inside a loop, returns will be converted to loop breaks targeting the loop scope.

use crate::Result;
use crate::cps::anf::HoistTarget;
use crate::cps::expr::CpsBind;
use crate::cps::{CpsContinuation, CpsCtx, CpsStep};
use crate::error::assert_no_attrs;
use crate::ir::{CpsSignal, CpsTerminator, CpsTerminatorSignal};
use syn::{ExprReturn, Ident};

impl CpsStep for ExprReturn {
    fn then(self, _continuation: CpsContinuation, ctx: &mut CpsCtx) -> Result<CpsContinuation> {
        lower_return(self, ctx)
    }
}

impl CpsBind for ExprReturn {
    fn and_then(
        self,
        _bind: Ident,
        _continuation: CpsContinuation,
        ctx: &mut CpsCtx,
    ) -> Result<CpsContinuation> {
        lower_return(self, ctx)
    }
}

fn lower_return(ret: ExprReturn, ctx: &mut CpsCtx) -> Result<CpsContinuation> {
    assert_no_attrs(ret.attrs)?;

    match ctx.current_loop() {
        None => ret.expr.step_with(ctx),
        Some(mut current_loop) => {
            current_loop.inner().log_returns();
            let from = current_loop.inner().loop_id();
            ret.expr.map(|e| *e).with_hoisting(
                |expr, _ctx| {
                    let signal = CpsTerminator::Signal(CpsTerminatorSignal {
                        from,
                        kind: CpsSignal::Return { val: expr },
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
    use crate::error::CpsError;
    use syn::parse_quote;

    #[test]
    fn return_with_attrs_rejected() {
        let mut expr: ExprReturn = parse_quote!(return 42);
        expr.attrs = vec![syn::parse_quote!(#[inline])];
        let err = expr.step().unwrap_err();
        assert!(matches!(*err, CpsError::UnallowedAttributes(..)));
    }
}
