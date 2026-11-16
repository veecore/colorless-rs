use crate::function::expand_fn_from;
use crate::util::exclude;
use crate::{
    config::Config,
    util::{add_generic, assert_not_const, rewrite_output, thunk, wrap_in_resolvable},
};
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::punctuated::Punctuated;
use syn::{
    Block, Expr, ExprAsync, ExprBlock, ExprCall, GenericArgument, ImplItem, ImplItemFn, ItemFn,
    ItemImpl, Path, PathArguments, Stmt, Type, parse_quote,
};

pub fn expand(config: Config, mut impl_item: ItemImpl) -> syn::Result<TokenStream> {
    let ty = match config
        .color
        .try_get_or_create_generic_color(&impl_item.generics)
    {
        Ok(generic) => {
            impl_item.generics = add_generic(impl_item.generics, &config.crate_path, &generic);
            impl_item.items = impl_item
                .items
                .into_iter()
                .map(|item| match item {
                    ImplItem::Fn(func) => {
                        expand_impl_trait_fn_generic(&config, func).map(ImplItem::Fn)
                    }
                    other => Ok(other),
                })
                .collect::<syn::Result<_>>()?;

            &parse_quote!(#generic)
        }
        Err(specific) => {
            impl_item.items = impl_item
                .items
                .into_iter()
                .map(|item| match item {
                    ImplItem::Fn(func) => {
                        expand_impl_trait_fn_specific(&config, func, specific).map(ImplItem::Fn)
                    }
                    other => Ok(other),
                })
                .collect::<syn::Result<_>>()?;

            specific
        }
    };

    if let Some((trait_path, _)) = &mut impl_item.trait_ {
        append_color_argument(trait_path, ty)?;
    } else if let Type::Path(s) = &mut *impl_item.self_ty {
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

fn expand_impl_trait_fn_generic(config: &Config, function: ImplItemFn) -> syn::Result<ImplItemFn> {
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

fn expand_impl_trait_fn_specific(
    config: &Config,
    mut function: ImplItemFn,
    color: &Type,
) -> syn::Result<ImplItemFn> {
    if exclude(&mut function.attrs) {
        return Ok(function);
    }

    assert_not_const(&function.sig)?;

    let crate_path = &config.crate_path;
    let is_async = function.sig.asyncness.take().is_some();

    function.sig.output = rewrite_output(
        function.sig.output,
        &config.crate_path,
        color,
        config.autocomplete,
    );

    let native = if is_async {
        Expr::Async(ExprAsync {
            attrs: vec![],
            async_token: Default::default(),
            capture: Some(Default::default()),
            modifiers: Default::default(),
            block: function.block,
        })
    } else {
        Expr::Block(ExprBlock {
            attrs: vec![],
            label: None,
            block: function.block,
        })
    };

    // Keep `return` scoped to the native implementation body instead of
    // letting it return from the transformed method, whose return type is
    // now `impl Colored<...>`.
    let colorize = Expr::Call(ExprCall {
        attrs: vec![],
        func: parse_quote!(#crate_path::__compiler::colorize),
        paren_token: Default::default(),
        args: Punctuated::from_iter([thunk(native, true)]),
    });

    let colorize = if config.autocomplete {
        wrap_in_resolvable(&config.crate_path, colorize)
    } else {
        colorize
    };

    function.block = Block {
        brace_token: Default::default(),
        stmts: vec![Stmt::Expr(colorize, None)],
    };

    Ok(function)
}
