use crate::function::expand_fn_from;
use crate::{config::Config, util::add_generic};
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{ItemFn, ItemTrait, Result, TraitItem, TraitItemFn, Visibility, parse_quote};

pub fn expand(config: Config, mut trait_item: ItemTrait) -> Result<TokenStream> {
    let color_generic = config
        .color
        .get_or_create_generic_color(&trait_item.generics)?;

    trait_item.generics = add_generic(trait_item.generics, &config.crate_path, &color_generic);

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
