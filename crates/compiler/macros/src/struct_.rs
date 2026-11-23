use crate::config::Config;
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{Fields, ItemStruct, parse_quote};

pub fn expand(mut config: Config, mut struct_: ItemStruct) -> syn::Result<TokenStream> {
    // add __C: ::colorless::Color = ::colorless::Identity
    config.or_add_generic(&mut struct_.generics, false); // todo: maybe later

    let color_generic = config.unwrap_generic_color()?;

    // add Phantom
    match &mut struct_.fields {
        // todo: choose an unused field name
        Fields::Named(named_fields) => named_fields.named.push(parse_quote! {
            __colorless_marker: ::core::marker::PhantomData::<#color_generic>
        }),
        Fields::Unnamed(unnamed_fields) => {
            unnamed_fields.unnamed.push(parse_quote! {
                ::core::marker::PhantomData::<#color_generic>
            });
        }
        Fields::Unit => {
            struct_.fields = Fields::Unnamed(parse_quote! {
                (::core::marker::PhantomData::<#color_generic>)
            })
        }
    };

    Ok(struct_.to_token_stream())
}
