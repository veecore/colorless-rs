//! Errors returned during CPS lowering.

use crate::Result;
use proc_macro2::TokenStream;
use quote::quote;
use syn::token::{Else, If};
use syn::{Attribute, ExprBreak, ExprContinue, ExprLet, ExprTryBlock, Macro};

/// Errors that can occur while lowering Rust syntax into the CPS IR.
#[derive(Debug, Clone)]
pub enum CpsError {
    /// A `break` expression whose target label is missing, out of scope, or
    /// would break through one or more loop boundaries to reach a labeled block.
    InvalidBreak(ExprBreak),

    /// A `continue` expression whose target label is missing or out of scope.
    InvalidContinue(ExprContinue),

    /// A `let` expression used outside of an `if` condition, where it is not
    /// permitted.
    LetExpr(ExprLet),

    /// An `if` expression used as a value is missing its `else` branch.
    MissingElse(If),

    /// A raw [`syn::Error`] produced by an upstream parse step.
    Other(syn::Error),

    /// A `try { … }` block expression is not yet supported.
    ///
    /// `try` blocks must execute in the caller's context and interact with
    /// the CPS continuation in ways that are not yet handled.
    TryBlock(ExprTryBlock),

    /// Attributes are not permitted on this expression inside the CPS IR.
    UnallowedAttributes(Vec<Attribute>),

    /// This macro invocation is not permitted in this position inside the CPS
    /// IR.  Only a known set of statement-position macros are allowed.
    UnallowedMacro(Macro),

    /// The `else` branch of a `let`-`else` could not be confirmed to diverge.
    /// The lowering requires a provably diverging branch (one that returns, breaks,
    /// continues, or panics via a known macro).
    UnclearDivergence(Else),

    /// An unparsed verbatim token stream is not permitted in the CPS IR.
    Verbatim(TokenStream),
}

/// Asserts that an attribute list is empty, or returns
/// [`CpsError::UnallowedAttributes`].
#[inline]
pub(crate) fn assert_no_attrs(attrs: Vec<Attribute>) -> Result<()> {
    // discarding the attr here but inert has it
    if attrs.is_empty()
        || attrs
            .iter()
            .any(|attr| attr.path().get_ident().is_some_and(|i| i == "allow"))
    {
        Ok(())
    } else {
        Err(CpsError::UnallowedAttributes(attrs).into())
    }
}

impl From<CpsError> for syn::Error {
    fn from(err: CpsError) -> Self {
        match err {
            CpsError::InvalidBreak(expr) => syn::Error::new_spanned(
                expr,
                "invalid `break`: the target label is missing, out of scope, or would break \
                 through one or more loops to reach a labeled block",
            ),
            CpsError::InvalidContinue(expr) => syn::Error::new_spanned(
                expr,
                "invalid `continue`: the target label is missing or out of scope",
            ),
            CpsError::LetExpr(expr) => syn::Error::new_spanned(
                expr,
                "`let` expressions are only permitted as conditions of `if` expressions",
            ),
            CpsError::MissingElse(if_) => {
                syn::Error::new_spanned(if_, "`if` used as a value must have an `else` branch")
            }
            CpsError::Other(e) => e,
            CpsError::TryBlock(block) => syn::Error::new_spanned(
                block,
                "`try { … }` block expressions are not yet supported in the CPS IR",
            ),
            CpsError::UnallowedAttributes(attrs) => syn::Error::new_spanned(
                quote!(#(#attrs)*),
                "attributes are not permitted on this expression in the CPS IR",
            ),
            CpsError::UnallowedMacro(mac) => syn::Error::new_spanned(
                mac,
                "this macro invocation is not permitted in this position in the CPS IR",
            ),
            CpsError::UnclearDivergence(expr) => syn::Error::new_spanned(
                expr,
                "the `else` branch of a `let`-`else` must clearly diverge (via `return`, \
                 `break`, `continue`, or a panicking macro); the lowering could not confirm \
                 that this branch does",
            ),
            CpsError::Verbatim(verbatim) => syn::Error::new_spanned(
                verbatim,
                "verbatim token streams are not permitted in the CPS IR",
            ),
        }
    }
}

impl From<Box<CpsError>> for syn::Error {
    fn from(err: Box<CpsError>) -> Self {
        syn::Error::from(*err)
    }
}

impl CpsError {
    /// Renders this error as a `compile_error!(…)` token stream, preserving
    /// the original span so the compiler points to the right source location.
    pub fn into_compile_error(self) -> TokenStream {
        let syn_err: syn::Error = self.into();
        syn_err.to_compile_error()
    }
}
