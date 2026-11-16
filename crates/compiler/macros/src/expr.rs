use crate::{
    config::{Color, Config, DropTiming},
    inert::{generate_inert_body, with_original_validation},
    util::wrap_in_resolvable,
};
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, Block, Result, Stmt, Token, Type, parse_quote};

pub struct ColorInput {
    pub config: Config,
    pub capture: Option<Token![move]>,
    pub block: Block,
}

pub fn expand(input: ColorInput) -> Result<TokenStream> {
    let config = input.config;
    let block = input.block;

    core::debug_assert_matches!(config.drop_timing, DropTiming::Relaxed);

    let color_type = match &config.color {
        Color::Generic(g) => {
            return Err(syn::Error::new_spanned(
                g,
                "`color! {}` does not take generic color",
            ));
        }
        Color::Specific(s) => s.clone(),
        Color::Auto => {
            parse_quote!(_)
        }
    };

    expand_block(block, &config, &color_type, input.capture.is_some())
        .map(ToTokens::into_token_stream)
}

pub fn expand_block(
    block: Block,
    config: &Config,
    color_type: &Type,
    capture: bool,
) -> Result<Block> {
    core::debug_assert_matches!(config.drop_timing, DropTiming::Relaxed);

    let crate_path = config.crate_path.clone();

    let mut codegen_config = colorless_codegen::Config {
        max_one_of_arms: 12,
        color_type: color_type.clone(),
        crate_path: parse_quote!(#crate_path::__compiler),
        // inert strips off the resolves... if it has none, then we could safely use it
        original_block_plain: Some(generate_inert_body(block.clone(), color_type, &crate_path)),
        declare_borrow_macros: true,
        no_capture: !capture,
        allow_redundant_pin: false,
        colorless_into_iterator: parse_quote!(#crate_path::iter::IntoColoredIterator::into_iter),
    };

    let folded_expr = codegen_config.generate_block(block)?;
    let body_expr = if config.autocomplete {
        wrap_in_resolvable(&crate_path, folded_expr)
    } else {
        folded_expr
    };

    // else it consumed it and there's no need for inert
    let emit = if let Some(inert) = codegen_config.original_block_plain {
        with_original_validation(&crate_path, body_expr, inert, true)
    } else {
        body_expr
    };

    Ok(Block {
        brace_token: Default::default(),
        stmts: vec![Stmt::Expr(emit, None)],
    })
}

impl Parse for ColorInput {
    fn parse(input: ParseStream) -> Result<Self> {
        // color! {move
        //     #![colorless(color = Async)]
        //
        //     println!()
        // }

        let content = input;
        let move_ = content.parse::<Option<Token![move]>>()?;
        let mut inner_attrs = content.call(Attribute::parse_inner)?;

        let config = if let Some(attr) = inner_attrs.pop() {
            if let Some(attr1) = inner_attrs.pop() {
                return Err(syn::Error::new_spanned(
                    attr1,
                    "multiple inner attributes are not supported",
                ));
            }

            if !attr.path().get_ident().is_some_and(|d| d == "colorless") {
                return Err(syn::Error::new_spanned(attr, "unsupported inner attribute"));
            }

            attr.parse_args()?
        } else {
            Config::default()
        };

        let stmts = content.call(Block::parse_within)?;

        Ok(ColorInput {
            config,
            capture: move_,
            block: Block {
                brace_token: Default::default(),
                stmts,
            },
        })
    }
}
