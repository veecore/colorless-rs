//! # `colorless-prepare`
//!
//! Conservative source AST preparation before Colorless's typed lowering.
//! Disambiguates shadowed bindings and infers explicit directives (`loop_state`).

use proc_macro::TokenStream;

mod bindings;
mod loops;

#[proc_macro_attribute]
pub fn prepare(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
