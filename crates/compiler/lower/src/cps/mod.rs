//! Lowers AST expressions, statements, and blocks into continuation-passing style.
//!
//! Subsystems:
//! - [`anf`]: A-Normal Form expression hoister ([`anf::Hoister`]) for sequencing sub-expressions.
//! - [`context`]: Lowering context, loop scopes, and control-flow target resolution.
//! - [`continuation`]: The continuation buffer ([`CpsContinuation`]).
//! - [`directive`]: Parser for compiler directives (such as `#[loop_state]`).
//! - [`expr`]: Expression lowering visitors (branches, loops, jumps, and leaf expressions).
//! - [`stmts`]: Statement sequencing and local binding transformations.

pub mod anf;
pub mod context;
pub mod continuation;
pub mod directive;
pub mod expr;
pub mod stmts;

use crate::Result;
use crate::ir::CpsTerminator;
use crate::ir::scoped::{CpsScopedContinuation, CpsTerminatorScopedContUse};
pub use context::CpsCtx;
pub use continuation::CpsContinuation;
use proc_macro2::Ident;

/// Foundational step in Continuation-Passing Style lowering.
///
/// Sequences `c` after `self`, discarding `self`'s computed value
/// (evaluating for side effects or as a statement).
pub trait CpsStep<T = CpsContinuation>: Sized {
    fn then(self, c: CpsContinuation, ctx: &mut CpsCtx) -> Result<T>;

    /// Sequences an empty continuation after `self` using a default context.
    #[inline]
    fn step(self) -> Result<T> {
        self.then(Default::default(), &mut CpsCtx::default())
    }

    /// Sequences an empty continuation after `self` using the provided context.
    #[inline]
    fn step_with(self, ctx: impl IntoCpsCtx) -> Result<T> {
        let mut ctx = ctx.into_cps_ctx();
        self.then(Default::default(), ctx.as_mut())
    }
}

/// Value-binding continuation step for an expression.
pub trait CpsBind<T = CpsContinuation>: CpsStep<T> {
    /// Binds `self`'s computed value to identifier `bind`, then proceeds with `continuation`.
    fn and_then(self, bind: Ident, continuation: CpsContinuation, ctx: &mut CpsCtx) -> Result<T>;

    #[inline]
    fn maybe_then(self, cont: Option<CpsScopedContinuation>, ctx: &mut CpsCtx) -> Result<T> {
        if let Some(cont) = cont {
            let bind = cont.binding.clone();
            let continuation =
                CpsContinuation::from(CpsTerminator::ContUse(CpsTerminatorScopedContUse {
                    scope_continuation: cont,
                }));
            match bind {
                Some(bind) => self.and_then(bind, continuation, ctx),
                None => self.then(continuation, ctx),
            }
        } else {
            self.step_with(ctx)
        }
    }
}

// Blanket impl target so callers can pass either a `Config` or an existing `&mut CpsCtx`.
pub trait IntoCpsCtx {
    fn into_cps_ctx(self) -> impl AsMut<CpsCtx>;
}

mod blanket_impls {
    use crate::cps::{CpsBind, CpsContinuation, CpsCtx, CpsStep, IntoCpsCtx};
    use crate::{Config, Result};
    use proc_macro2::Ident;

    impl<T, M> CpsStep<T> for Box<M>
    where
        M: CpsStep<T>,
    {
        #[inline]
        fn then(self, c: CpsContinuation, ctx: &mut CpsCtx) -> Result<T> {
            (*self).then(c, ctx)
        }
    }

    impl<T, E: CpsBind<T>> CpsBind<T> for Box<E> {
        #[inline]
        fn and_then(
            self,
            bind: Ident,
            continuation: CpsContinuation,
            ctx: &mut CpsCtx,
        ) -> Result<T> {
            (*self).and_then(bind, continuation, ctx)
        }
    }

    impl<M: CpsStep> CpsStep for Option<M> {
        #[inline]
        fn then(self, continuation: CpsContinuation, ctx: &mut CpsCtx) -> Result<CpsContinuation> {
            match self {
                Some(val) => val.then(continuation, ctx),
                None => Ok(continuation),
            }
        }
    }

    impl IntoCpsCtx for Config {
        #[inline]
        fn into_cps_ctx(self) -> impl AsMut<CpsCtx> {
            CpsCtx::new(self)
        }
    }

    impl IntoCpsCtx for CpsCtx {
        #[inline]
        fn into_cps_ctx(self) -> impl AsMut<CpsCtx> {
            self
        }
    }

    impl IntoCpsCtx for &mut CpsCtx {
        #[inline]
        fn into_cps_ctx(self) -> impl AsMut<CpsCtx> {
            self
        }
    }

    impl AsMut<CpsCtx> for CpsCtx {
        fn as_mut(&mut self) -> &mut CpsCtx {
            self
        }
    }
}

mod macros {
    #[macro_export]
    macro_rules! derive_cps_bind_scope_delegate {
        ($ty:ident => $lower_fn:ident / $end:ident) => {
            derive_cps_bind_scope_delegate!($ty => $lower_fn -> CpsContinuation / $end);
        };
        ($ty:ident => $lower_fn:ident -> $tr:ident / $end:ident) => {
            impl CpsStep<$tr> for $ty {
                fn then(self, continuation: $crate::cps::CpsContinuation, ctx: &mut $crate::cps::CpsCtx) -> Result<$tr> {
                    let scope_end = continuation.$end(None, ctx);
                    $lower_fn(self, scope_end, ctx)
                }
            }

            impl CpsBind<$tr> for $ty {
                fn and_then(
                    self,
                    bind: Ident,
                    continuation: $crate::cps::CpsContinuation,
                    ctx: &mut $crate::cps::CpsCtx,
                ) -> Result<$tr> {
                    let scope_end = continuation.$end(Some(bind), ctx);
                    $lower_fn(self, scope_end, ctx)
                }
            }
        };
    }
}
