use crate::config::Config;
use crate::function::expand_fn_from;
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    GenericArgument, ImplItem, ImplItemFn, ItemFn, ItemImpl, Path, PathArguments, Type, parse_quote,
};

pub fn expand(mut config: Config, mut impl_item: ItemImpl) -> syn::Result<TokenStream> {
    config.or_add_generic(&mut impl_item.generics, false);

    impl_item.items = impl_item
        .items
        .into_iter()
        .map(|item| match item {
            ImplItem::Fn(func) => expand_impl_trait_fn(&config, func).map(ImplItem::Fn),
            other => Ok(other),
        })
        .collect::<syn::Result<_>>()?;

    let ty = match config.unwrap_color() {
        Ok(g) => &parse_quote!(#g),
        Err(s) => s,
    };

    if let Some((trait_path, _)) = &mut impl_item.trait_ {
        append_color_argument(trait_path, ty)?;
    } else if let Type::Path(s) = &mut *impl_item.self_ty
        && cfg!(feature = "color-impl")
    {
        append_color_argument(&mut s.path, ty)?;
    }

    Ok(impl_item.to_token_stream())
}

pub fn append_color_argument(path: &mut Path, color: &Type) -> syn::Result<()> {
    let Some(segment) = path.segments.last_mut() else {
        return Err(syn::Error::new_spanned(path, "expected a trait path"));
    };

    match &mut segment.arguments {
        PathArguments::None => {
            let arguments: syn::AngleBracketedGenericArguments = parse_quote!(<#color>);
            segment.arguments = PathArguments::AngleBracketed(arguments);
        }
        PathArguments::AngleBracketed(arguments) => {
            arguments.args.push(GenericArgument::Type(color.clone()));
        }
        PathArguments::Parenthesized(arguments) => {
            return Err(syn::Error::new_spanned(
                arguments,
                "a color argument cannot be added to parenthesized path arguments",
            ));
        }
    }
    Ok(())
}

fn expand_impl_trait_fn(config: &Config, function: ImplItemFn) -> syn::Result<ImplItemFn> {
    let ImplItemFn {
        attrs,
        vis,
        modifiers,
        sig,
        block,
    } = function;

    let func = ItemFn {
        attrs,
        vis,
        modifiers,
        sig,
        block: Box::new(block),
    };

    let func = expand_fn_from(config, func)?;

    Ok(ImplItemFn {
        attrs: func.attrs,
        vis: func.vis,
        modifiers: func.modifiers,
        sig: func.sig,
        block: *func.block,
    })
}
