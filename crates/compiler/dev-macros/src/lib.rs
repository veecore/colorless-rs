mod attrs;

use crate::attrs::{CodegenConfig, Config};
use colorless_lower::cps::CpsStep;
use colorless_lower::ir::{CpsBlock, IntoSyn};
use proc_macro::TokenStream;
use quote::ToTokens;
use syn::{Block, ItemFn, ReturnType, Signature, Stmt, parse_macro_input, parse_quote};

/// Proc-macro attribute that lowers a function body through the Colorless CPS IR.
///
/// Emits both the lowered CPS function and an inert copy of the original function inside
/// a `const _: () = { ... }` block so that `rustc` enforces all native type checks (such as
/// `let-else` divergence, borrow checking, and lifetimes) without runtime overhead.
#[proc_macro_attribute]
pub fn cps_fn(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);

    cps_fn_impl(_attr, input_fn).unwrap_or_else(|err| err.to_compile_error().into())
}

fn cps_fn_impl(attr: TokenStream, mut input_fn: ItemFn) -> syn::Result<TokenStream> {
    let config = Config::from_attributes(attr.into())?;

    let colorless_cps_config = config.as_cps_config();

    let original_fn = input_fn.clone();
    let block: CpsBlock = input_fn.block.step_with(colorless_cps_config)?;
    let block: Block = block.into_syn();
    input_fn.block = block.into();
    let lowered = input_fn;

    let mut expanded = quote::quote! {
        #[allow(unused_parens, unused_braces)]
        #lowered
    };

    if config.emit_inert {
        let inert = quote::quote! {
            const _: () = {
                #[allow(dead_code)]
                #original_fn
            };
        };

        expanded.extend(inert);
    }

    if let Some(original_fn) = config.emit_original(original_fn) {
        let inert = quote::quote!(#original_fn);

        expanded.extend(inert);
    }

    Ok(expanded.into())
}

/// Proc-macro function-like macro that lowers a block expression into a CPS expression.
#[proc_macro]
pub fn cps_expr(input: TokenStream) -> TokenStream {
    let input_block = parse_macro_input!(input as Block);
    let output: Result<CpsBlock, _> = input_block.step();

    match output {
        Ok(func) => {
            let f: Block = func.into_syn();
            f.into_token_stream().into()
        }
        Err(err) => err.into_compile_error().into(),
    }
}

#[proc_macro_attribute]
pub fn codegen(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input_fn = parse_macro_input!(item as ItemFn);
    match CodegenConfig::from_attributes(attr.into()) {
        Ok(_) => {}
        Err(error) => return error.to_compile_error().into(),
    }
    let mut codegen_config = colorless_codegen::Config::default();

    // codegen_config.original_block = Some(*input_fn.block.clone());

    match codegen_config.generate_block(*input_fn.block) {
        Ok(folded) => {
            input_fn.block = Block {
                brace_token: Default::default(),
                stmts: vec![Stmt::Expr(folded, None)],
            }
            .into();
            input_fn.sig = make_colorless(input_fn.sig);
            input_fn.into_token_stream().into()
        }
        Err(err) => err.to_compile_error().into(),
    }
}

// /// Experimental code-generation path used to validate compiler architecture
// /// without changing the production `#[codegen]` expansion.
// #[proc_macro_attribute]
// pub fn codex_codegen(_attr: TokenStream, item: TokenStream) -> TokenStream {
//     let mut input_fn = parse_macro_input!(item as ItemFn);
//     match colorless_codex_codegen::Config::default().generate_block(*input_fn.block) {
//         Ok(folded) => {
//             input_fn.block = Block {
//                 brace_token: Default::default(),
//                 stmts: vec![Stmt::Expr(folded, None)],
//             }
//             .into();
//             input_fn.sig = make_colorless(input_fn.sig);
//             input_fn.into_token_stream().into()
//         }
//         Err(err) => err.to_compile_error().into(),
//     }
// }

fn make_colorless(mut sig: Signature) -> Signature {
    sig.generics
        .params
        .push(parse_quote!(__C: ::colorless::Color));
    let original_output = match sig.output {
        ReturnType::Default => {
            parse_quote!(())
        }
        ReturnType::Type(_, ty) => ty,
    };
    sig.output = parse_quote!(-> impl ::colorless::Colored<__C, Output = #original_output>);
    sig
}
