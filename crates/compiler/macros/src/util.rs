use proc_macro2::Ident;
use syn::punctuated::Punctuated;
use syn::{
    Attribute, Block, Expr, ExprBlock, ExprClosure, ExprStruct, FieldValue, GenericParam, Generics,
    ReturnType, Signature, Type, TypeParam, parse_quote,
};

pub fn add_generic(
    generics: &mut Generics,
    crate_path: &syn::Path,
    color_generic: &Ident,
    default: bool,
    erase: bool,
) {
    let color = if erase {
        // fix, add `'__erase` bound by that
        quote::quote! {
            #crate_path::Color + #crate_path::Erase + 'static
        }
    } else {
        quote::quote! {
            #crate_path::Color
        }
    };
    let parameter: TypeParam = if default {
        parse_quote!(#color_generic: #color = ::colorless::Identity)
    } else {
        parse_quote!(#color_generic: #color)
    };
    generics.params.push(GenericParam::Type(parameter));
}

pub fn rewrite_output(
    mut output: ReturnType,
    crate_path: &syn::Path,
    color: &Type,
    use_autocomplete: bool,
    // is_dyn: bool,
) -> ReturnType {
    let original_output = match output {
        ReturnType::Default => parse_quote!(()),
        ReturnType::Type(_, ty) => ty,
    };

    // let path = if is_dyn {
    //     parse_quote!(<#color as ::#crate_path::Erase>::Erased)
    // } else {
    //
    // };

    if use_autocomplete {
        output = parse_quote!(-> #crate_path::Resolvable<impl #crate_path::Colored<#color, Output = #original_output>>);
    } else {
        output = parse_quote!(-> impl #crate_path::Colored<#color, Output = #original_output>);
    }

    output
}

pub fn exclude(func: &mut Vec<Attribute>) -> bool {
    let attrs = func.len();
    func.retain(|attr| attr.path().get_ident().is_some_and(|i| i != "exclude"));

    attrs != func.len()
}

pub fn assert_not_const(sig: &Signature) -> syn::Result<()> {
    // yet because const Trait isn't stable...
    if let Some(constness) = sig.constness {
        return Err(syn::Error::new(
            constness.span,
            "a `#[colorless]` function cannot yet be `const`",
        ));
    }
    Ok(())
}

pub fn assert_not_async(sig: &Signature) -> syn::Result<()> {
    if let Some(asyncness) = sig.asyncness {
        return Err(syn::Error::new(
            asyncness.span,
            "a `#[colorless]` function is already generic over execution; remove `async` and use `.resolve`",
        ));
    }

    Ok(())
}

pub fn wrap_in_resolvable(crate_path: &syn::Path, expr: Expr) -> Expr {
    Expr::Struct(ExprStruct {
        attrs: vec![],
        qself: None,
        path: parse_quote!(#crate_path::Resolvable),
        brace_token: Default::default(),
        fields: Punctuated::from_iter([FieldValue {
            attrs: vec![],
            member: parse_quote!(resolve),
            colon_token: Some(Default::default()),
            expr,
        }]),
        dot2_token: None,
        rest: None,
    })
}

// we don't want this block targeting the transformed
pub fn thunk(block: Expr, capture: bool) -> Expr {
    Expr::Closure(ExprClosure {
        attrs: vec![],
        lifetimes: None,
        modifiers: Default::default(),
        constness: None,
        asyncness: None,
        capture: capture.then(Default::default),
        inputs_begin: Default::default(),
        inputs: Default::default(),
        inputs_end: Default::default(),
        output: ReturnType::Default,
        body: block.into(),
    })
}

pub fn thunk_block(block: Block, capture: bool) -> Expr {
    thunk(
        Expr::Block(ExprBlock {
            attrs: vec![],
            label: None,
            block,
        }),
        capture,
    )
}
