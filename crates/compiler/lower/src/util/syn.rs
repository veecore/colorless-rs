//! Helper constructors for standard `syn` AST nodes (blocks, paths, break, if/else).

use proc_macro2::Span;
use syn::punctuated::Punctuated;
use syn::{Expr, ExprLit, ExprTuple, Lit, LitBool, token};

#[inline]
#[must_use]
pub fn unit_syn_expr() -> Expr {
    Expr::Tuple(ExprTuple {
        attrs: vec![],
        paren_token: token::Paren::default(),
        elems: Punctuated::new(),
    })
}

#[inline]
#[must_use]
pub fn make_bool(value: bool) -> Expr {
    Expr::Lit(ExprLit {
        attrs: vec![],
        lit: Lit::Bool(LitBool {
            value,
            span: Span::call_site(),
        }),
    })
}
