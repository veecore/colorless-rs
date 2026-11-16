//! CPS lowering for atomic leaf expressions that never diverge (Literals, Paths, Idents).

use crate::derive_cps_bind_simple;
use syn::{ExprAsync, ExprClosure, ExprConst, ExprInfer, ExprLit, ExprPath};

derive_cps_bind_simple! {
    ExprAsync,
    ExprClosure,
    ExprConst,
    ExprInfer,
    ExprLit,
    ExprPath,
}

mod macros {
    /// Implements [`CpsStep`](crate::cps::CpsStep) and [`CpsBind`](crate::cps::expr::CpsBind)
    /// for atomic expressions that do not contain sub-expressions and cannot diverge
    /// on their own (e.g. literals, paths).
    #[macro_export]
    macro_rules! derive_cps_bind_simple {
        (
            $($ty:ty,)*
        ) => {
            $(
                derive_cps_bind_simple!($ty);
            )*
        };
        ($ty:ty) => {
            impl $crate::cps::CpsStep for $ty {
                #[inline]
                fn then(
                    self,
                    continuation: $crate::cps::CpsContinuation,
                    _ctx: &mut $crate::cps::CpsCtx,
                ) -> $crate::Result<$crate::cps::CpsContinuation> {
                    let cps = $crate::ir::IntoCps::into_cps(self)?;
                    Ok($crate::cps::expr::prepend_expr_stmt(cps, continuation))
                }
            }

            impl $crate::cps::CpsBind for $ty {
                #[inline]
                fn and_then(
                    self,
                    bind: syn::Ident,
                    continuation: $crate::cps::CpsContinuation,
                    _ctx: &mut $crate::cps::CpsCtx,
                ) -> $crate::Result<$crate::cps::CpsContinuation> {
                    let cps = $crate::ir::IntoCps::into_cps(self)?;
                    Ok($crate::cps::expr::bind_expr_and_prepend_stmt(bind, cps, continuation))
                }
            }
        };
    }
}
