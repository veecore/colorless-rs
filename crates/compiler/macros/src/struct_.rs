use crate::config::Config;
use crate::util::add_color_generic;
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{Fields, ItemStruct, parse_quote};

pub fn expand(config: Config, mut struct_: ItemStruct) -> syn::Result<TokenStream> {
    let color_generic = config
        .color
        .get_or_create_generic_color(&struct_.generics)?;

    // add __C: ::colorless::Color = ::colorless::Identity and Phantom
    let crate_path = &config.crate_path;
    struct_.generics = add_color_generic(struct_.generics, crate_path, &color_generic, true);

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
