use crate::util::{exclude, thunk, wrap_in_resolvable};
use crate::{
    config::{Config, DropTiming},
    expr::expand_block,
    util::{assert_not_async, assert_not_const, rewrite_output},
};
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::punctuated::Punctuated;
use syn::{Block, Expr, ExprAsync, ExprBlock, ExprCall, Ident, Stmt, Type};
use syn::{ItemFn, Signature, parse_quote};

pub fn expand(mut config: Config, mut func: ItemFn) -> syn::Result<TokenStream> {
    config.or_add_generic(&mut func.sig.generics, false);

    expand_fn(&config, func).map(ToTokens::into_token_stream)
}

// expects that generic is predeclared
pub fn expand_fn_from(config: &Config, mut func: ItemFn) -> syn::Result<ItemFn> {
    if exclude(&mut func.attrs) {
        Ok(func)
    } else {
        expand_fn(config, func)
    }
}

fn expand_fn(config: &Config, func: ItemFn) -> syn::Result<ItemFn> {
    match config.unwrap_color() {
        Ok(color_generic) => expand_fn_generic(config, func, color_generic),
        Err(s) => expand_fn_specific(config, func, s),
    }
}

fn expand_fn_generic(
    config: &Config,
    mut func: ItemFn,
    color_generic: &Ident,
) -> syn::Result<ItemFn> {
    validate_generic_signature(&func.sig)?;

    assert!(matches!(config.drop_timing, DropTiming::Relaxed));

    let emit = expand_block(*func.block, config, &parse_quote!(#color_generic), true)?;

    func.sig.output = rewrite_output(
        func.sig.output,
        &config.crate_path,
        &parse_quote!(#color_generic),
        config.autocomplete,
    );

    func.block = Box::new(emit);

    Ok(func)
}

fn expand_fn_specific(
    config: &Config,
    mut func: ItemFn,
    color_specific: &Type,
) -> syn::Result<ItemFn> {
    // if config.dev_mode {
    //     // it's kinda useless here
    // }
    assert_not_const(&func.sig)?;

    let crate_path = &config.crate_path;
    let is_async = func.sig.asyncness.take().is_some();

    func.sig.output = rewrite_output(
        func.sig.output,
        &config.crate_path,
        color_specific,
        config.autocomplete,
    );

    let native = if is_async {
        Expr::Async(ExprAsync {
            attrs: vec![],
            async_token: Default::default(),
            capture: Some(Default::default()),
            modifiers: Default::default(),
            block: *func.block,
        })
    } else {
        Expr::Block(ExprBlock {
            attrs: vec![],
            label: None,
            block: *func.block,
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

    func.block = Block {
        brace_token: Default::default(),
        stmts: vec![Stmt::Expr(colorize, None)],
    }
    .into();

    Ok(func)
}

// just a light check for better diagnosis... rustc would catch these anyway
pub fn validate_generic_signature(sig: &Signature) -> syn::Result<()> {
    assert_not_async(sig)?;
    assert_not_const(sig)
}
