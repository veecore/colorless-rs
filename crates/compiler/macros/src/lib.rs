//! # `colorless-macros`
//!
//! Public procedural macros for Colorless: `#[colorless]`, `#[colored]`, and `color!`.

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn colorless(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro_attribute]
pub fn colored(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro]
pub fn color(input: TokenStream) -> TokenStream {
    input
}
