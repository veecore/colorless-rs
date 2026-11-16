//! A-Normal Form (ANF) hoisting for potentially-diverging sub-expressions.
//!
//! When a compound expression like `[a(), b(), c()]` contains sub-expressions
//! that may diverge (via `return`, `?`, early exits), each sub-expression must
//! be evaluated and potentially intercepted before we can build the compound.
//!
//! [`Hoister`] collects sub-expressions into a stack; [`HoistTarget`] describes
//! what can be pushed onto it.  After all sub-expressions are registered.

use crate::ir::CpsTempHandoff;
use crate::{
    Result,
    cps::context::CpsCtx,
    cps::{CpsBind, CpsContinuation},
    ir::CpsFieldValue,
    ir::expr::CpsExpr,
    ir::expr::CpsInert,
};
use proc_macro2::Ident;
use syn::Expr;
use syn::punctuated::{Pair, Punctuated};

mod expr;
mod inert;

/// Accumulates sub-expressions that need to be evaluated before the enclosing
/// compound can be assembled.  Sequences them left-to-right using `let`-bindings
/// once [`finish`](Hoister::finish) is called.
#[must_use]
#[derive(Default)]
pub struct Hoister {
    pending: Vec<(Expr, Ident)>,
}

impl Hoister {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a single sub-expression for sequential evaluation, returning a
    /// placeholder [`CpsTempHandoff`] that stands in for its computed value.
    #[inline]
    #[must_use]
    pub fn hoist(&mut self, expr: Expr, ctx: &mut CpsCtx) -> CpsTempHandoff {
        let placeholder = ctx.next_temp_ident("tmp_e");
        self.pending.push((expr, placeholder.clone()));
        CpsTempHandoff { ident: placeholder }
    }

    #[inline]
    #[must_use]
    pub fn hoist_as_inert(&mut self, expr: Expr, ctx: &mut CpsCtx) -> CpsInert {
        self.hoist(expr, ctx).into()
    }

    #[inline]
    #[must_use]
    pub fn hoist_as_expr(&mut self, expr: Expr, ctx: &mut CpsCtx) -> CpsExpr {
        self.hoist_as_inert(expr, ctx).into()
    }

    #[inline]
    pub fn finish(
        self,
        mut continuation: CpsContinuation,
        ctx: &mut CpsCtx,
    ) -> Result<CpsContinuation> {
        for (expr, ident) in self.pending.into_iter().rev() {
            continuation = expr.and_then(ident, continuation, ctx)?;
        }
        Ok(continuation)
    }
}

/// Marks a type that can be pushed onto a [`Hoister`].
///
/// For `Expr` this replaces non-atomic expressions with a fresh temporary identifier
/// in the compound AST node and schedules the original expression for sequential
/// evaluation. Variables already in normal form (single-segment paths) are reused
/// directly to prevent artificial moves of non-`Copy` types.
/// Container types (`Box`, `Option`, `Punctuated`) delegate element-wise.
pub trait HoistTarget<T> {
    fn hoist_into(self, hoister: &mut Hoister, ctx: &mut CpsCtx) -> Result<T>;

    #[inline]
    fn with_hoisting<F, C, Ctx>(self, f: F, mut ctx: Ctx) -> Result<CpsContinuation>
    where
        Self: Sized,
        Ctx: AsMut<CpsCtx>,
        F: FnOnce(T, &mut Ctx) -> Result<C>,
        C: Into<CpsContinuation>,
    {
        let mut hoister = Hoister::new();
        let me = self.hoist_into(&mut hoister, ctx.as_mut())?;
        let buf = f(me, &mut ctx)?.into();
        hoister.finish(buf, ctx.as_mut())
    }
}

trait IsInert {}
impl IsInert for CpsInert {}
// CpsFieldValue is made of inert
impl IsInert for CpsFieldValue {}

impl<T, P, H> HoistTarget<Punctuated<H, P>> for Punctuated<T, P>
where
    H: IsInert,
    T: HoistTarget<H>,
    P: Default,
{
    #[inline]
    fn hoist_into(self, hoister: &mut Hoister, ctx: &mut CpsCtx) -> Result<Punctuated<H, P>> {
        self.into_pairs()
            .map(|elem| {
                let (v, p) = elem.into_tuple();
                v.hoist_into(hoister, ctx).map(|r| Pair::new(r, p))
            })
            .collect()
    }
}

impl<H, T: HoistTarget<H>> HoistTarget<Box<H>> for Box<T> {
    #[inline]
    fn hoist_into(self, hoister: &mut Hoister, ctx: &mut CpsCtx) -> Result<Box<H>> {
        (*self).hoist_into(hoister, ctx).map(Into::into)
    }
}

impl<H, T: HoistTarget<H>> HoistTarget<Option<H>> for Option<T> {
    #[inline]
    fn hoist_into(self, hoister: &mut Hoister, ctx: &mut CpsCtx) -> Result<Option<H>> {
        match self {
            None => Ok(None),
            Some(s) => s.hoist_into(hoister, ctx).map(Some),
        }
    }
}

mod macros {
    #[macro_export]
    macro_rules! derive_hoist_target {
        // For compound inert
        (
            $syn_ty:ident {
                _ $attrs:ident,
                $(
                    $( (pub $expand:ident) )?
                    $( $plain:ident )?,
                )*
            } -> {$variant:ident($body:ident)}
        ) => {
            derive_hoist_target! {
                $syn_ty {
                    _ $attrs,
                    $(
                        $( (pub $expand) )?
                        $( $plain )?,
                    )*
                } -> {CpsInert::$variant($body)}
            }

            derive_hoist_target! {
                $syn_ty {
                    _ $attrs,
                    $(
                        $( (pub $expand) )?
                        $( $plain )?,
                    )*
                } -> {CpsExpr::Inert($body)}
            }
        };
        (
            $syn_ty:ident {
                _ $attrs:ident,
                $(
                    $( (pub $expand:ident) )?
                    $( $plain:ident )?,
                )*
            } -> {$enum_:ident::$variant:ident($body:ident)}
        ) => {
            impl $crate::cps::anf::HoistTarget<$enum_> for $syn_ty {

                #[inline]
                fn hoist_into(
                    self,
                    hoister: &mut $crate::cps::anf::Hoister,
                    ctx: &mut $crate::cps::CpsCtx) -> $crate::Result<$enum_> {
                    let $syn_ty {
                        $attrs,
                        $( $( $expand, )?
                        $( $plain, )? )*
                    } = self;
                    $crate::error::assert_no_attrs($attrs)?;
                    $(
                        $(
                            let $expand
                                = $crate::cps::anf::HoistTarget::hoist_into($expand, hoister, ctx)?;
                        )?
                        $(
                            let $plain = $plain;
                        )?
                    )*
                    Ok($enum_::$variant($body {
                        $(
                            $( $expand )?
                            $( $plain )?,
                        )*
                    }.into()))
                }
            }
        };
    }

    #[macro_export]
    macro_rules! derive_hoist_target_placeholder {
        (
            $($ty:ident,)+
        ) => {
             $(
                 derive_hoist_target_placeholder!(@inert $ty);
                 derive_hoist_target_placeholder!(@expr $ty);
             )+
        };
        (
            @all_inert
            $($ty:ident,)+
        ) => {
             $(
                 derive_hoist_target_placeholder!(@inert $ty);
             )+
        };
        (
            @inert $ty:ident
        ) => {
            impl $crate::cps::anf::HoistTarget<$crate::ir::CpsInert> for $ty {
                #[inline]
                fn hoist_into(self, hoister: &mut $crate::cps::anf::Hoister, ctx: &mut $crate::cps::CpsCtx) -> $crate::Result<$crate::ir::CpsInert> {
                    Ok(hoister.hoist_as_inert(match_expr!($ty)(self), ctx))
                }
            }
        };
        (
            @expr $ty:ident
        ) => {
            impl $crate::cps::anf::HoistTarget<$crate::ir::CpsExpr> for $ty {
                #[inline]
                fn hoist_into(self, hoister: &mut $crate::cps::anf::Hoister, ctx: &mut $crate::cps::CpsCtx) -> $crate::Result<$crate::ir::CpsExpr> {
                    Ok(hoister.hoist_as_expr(match_expr!($ty)(self), ctx))
                }
            }
        }
    }

    #[macro_export]
    macro_rules! derive_hoist_target_inert {
        (
            $($variant:ident($ty:ty)),+
        ) => {
             $(
                impl $crate::cps::anf::HoistTarget<$crate::ir::CpsInert> for $ty {
                    #[inline]
                    fn hoist_into(
                        self,
                        _hoister: &mut $crate::cps::anf::Hoister,
                        _ctx: &mut $crate::cps::CpsCtx) -> $crate::Result<$crate::ir::CpsInert> {
                        let i = $crate::ir::IntoCps::into_cps(self)?;
                        Ok($crate::ir::CpsInert::$variant(i))
                    }
                }

                impl $crate::cps::anf::HoistTarget<$crate::ir::CpsExpr> for $ty {
                    #[inline]
                    fn hoist_into(
                        self,
                        hoister: &mut $crate::cps::anf::Hoister,
                        ctx: &mut $crate::cps::CpsCtx,
                    ) -> $crate::Result<$crate::ir::CpsExpr> {
                        // hoist as inert, then convert to expr
                        self.hoist_into(hoister, ctx)
                            .map(|i| $crate::ir::CpsExpr::Inert(i))
                    }
                }
             )+
        };
    }
}
