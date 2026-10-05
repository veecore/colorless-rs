//! A-Normal Form (ANF) hoisting for potentially-diverging sub-expressions.

use crate::cps::context::CpsCtx;
use proc_macro2::Ident;
use syn::Expr;

#[must_use]
#[derive(Default)]
pub struct Hoister {
    pending: alloc::vec::Vec<(Expr, Ident)>,
}

impl Hoister {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn hoist(&mut self, expr: Expr, ctx: &mut CpsCtx) -> Ident {
        let ident = ctx.next_temp_ident("tmp_e");
        self.pending.push((expr, ident.clone()));
        ident
    }
}

extern crate alloc;
