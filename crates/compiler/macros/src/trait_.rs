use crate::config::Config;
use crate::function::expand_fn_from;
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{ItemFn, ItemTrait, Result, TraitItem, TraitItemFn, Visibility, parse_quote};

pub fn expand(mut config: Config, mut trait_item: ItemTrait) -> Result<TokenStream> {
    config.or_add_generic(&mut trait_item.generics, false);

    trait_item.items = trait_item
        .items
        .into_iter()
        .map(|item| match item {
            TraitItem::Fn(func) => expand_trait_fn(&config, func).map(TraitItem::Fn),
            other => Ok(other),
        })
        .collect::<Result<_>>()?;

    Ok(trait_item.to_token_stream())
}

fn expand_trait_fn(config: &Config, function: TraitItemFn) -> Result<TraitItemFn> {
    let TraitItemFn {
        attrs,
        modifiers,
        sig,
        default,
        semi_token,
    } = function;

    let is_default = default.is_some();

    let func = ItemFn {
        attrs,
        vis: Visibility::Inherited,
        modifiers,
        sig,
        block: default.unwrap_or_else(|| parse_quote!({})).into(),
    };

    let func = expand_fn_from(config, func)?;

    Ok(TraitItemFn {
        attrs: func.attrs,
        modifiers: func.modifiers,
        sig: func.sig,
        default: if is_default { Some(*func.block) } else { None },
        semi_token,
    })
}
