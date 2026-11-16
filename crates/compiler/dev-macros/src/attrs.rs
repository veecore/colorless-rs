use colorless_lower::Config as ColorlessCpsConfig;
use proc_macro2::TokenStream;
use quote::format_ident;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Expr, ExprPath, ItemFn, Lit, Token, parse_quote};

pub struct CodegenConfig;

#[derive(Default)]
pub struct Config {
    pub emit_original: Option<EmitOriginalAs>,
    pub emit_inert: bool,
    pub allow_all_macros: bool,
}

#[derive(Default)]
pub enum EmitOriginalAs {
    Provided(String),
    #[default]
    Derived,
}

const HELP: &str = "\
                Expect #[cps_fn(emit_original /*= optional_name*/, emit_inert /*= true*/, allow_all_macros = /*= true*/)]\
            ";

impl Config {
    pub(crate) fn from_attributes(attr: TokenStream) -> syn::Result<Config> {
        let attrs: Punctuated<Expr, Token![,]> = parse_quote!(#attr);

        let mut config = Self::default();
        for attr in attrs {
            match attr {
                Expr::Assign(assign) => {
                    if !assign.attrs.is_empty() {
                        return Err(syn::Error::new(
                            assign.span(),
                            format!("unsupported attribute. {HELP}"),
                        ));
                    }

                    match (&*assign.left, &*assign.right) {
                        (Expr::Path(left), v) => set_attr_pair(&mut config, left, Some(v))?,
                        _ => {
                            return Err(syn::Error::new(
                                assign.span(),
                                format!("unsupported attribute. {HELP}"),
                            ));
                        }
                    }
                }
                Expr::Path(var) => set_attr_pair(&mut config, &var, None)?,
                _ => {
                    return Err(syn::Error::new(
                        attr.span(),
                        format!("unsupported argument format. {HELP}"),
                    ));
                }
            }
        }

        Ok(config)
    }

    pub fn as_cps_config(&self) -> ColorlessCpsConfig {
        let mut colorless_cps_config = ColorlessCpsConfig::default();
        if self.allow_all_macros {
            colorless_cps_config = colorless_cps_config.allow_all_macros()
        }
        if self.emit_inert {
            colorless_cps_config.allow_maybe_diverging_in_let_else = true;
        }
        colorless_cps_config.auto_inject_loop_state = false;

        colorless_cps_config
    }

    pub fn emit_original(&self, mut original: ItemFn) -> Option<ItemFn> {
        let new_name = match &self.emit_original {
            None => return None,
            Some(EmitOriginalAs::Derived) => {
                format_ident!("{}_original", original.sig.ident)
            }
            Some(EmitOriginalAs::Provided(name)) => {
                format_ident!("{}", name)
            }
        };
        original.sig.ident = new_name;
        Some(original)
    }
}

impl CodegenConfig {
    pub(crate) fn from_attributes(attr: TokenStream) -> syn::Result<Self> {
        let attrs: Punctuated<Expr, Token![,]> = parse_quote!(#attr);
        if attrs.is_empty() {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "colorless currently requires `drop_timing = \"relaxed\"`; transformed values may be destroyed earlier or later than their source scope and implicit sibling destruction order may differ",
            ));
        }

        let mut acknowledged = false;

        for attr in attrs {
            let Expr::Assign(assign) = attr else {
                return Err(syn::Error::new(
                    attr.span(),
                    "expected `drop_timing = \"relaxed\"`",
                ));
            };
            let Expr::Path(name) = &*assign.left else {
                return Err(syn::Error::new(
                    assign.left.span(),
                    "expected `drop_timing`",
                ));
            };
            if name
                .path
                .get_ident()
                .is_none_or(|ident| ident != "drop_timing")
            {
                return Err(syn::Error::new(name.span(), "unsupported codegen option"));
            }

            if acknowledged {
                return Err(syn::Error::new(
                    name.span(),
                    "`drop_timing` may only be specified once",
                ));
            }
            if parse_string(&assign.right)? != "relaxed" {
                return Err(syn::Error::new(
                    assign.right.span(),
                    "the only supported destruction policy is `drop_timing = \"relaxed\"`",
                ));
            }
            acknowledged = true;
        }

        Ok(Self)
    }
}

fn set_attr_pair(config: &mut Config, var: &ExprPath, value: Option<&Expr>) -> syn::Result<()> {
    match parse_var(var)? {
        Var::EmitOriginal => {
            config.emit_original = match value {
                None => Some(EmitOriginalAs::Derived),
                Some(s) => Some(EmitOriginalAs::Provided(parse_string(s)?)),
            };
        }
        Var::EmitInert => {
            config.emit_inert = parse_bool_or_true(value)?;
        }
        Var::AllowAllMacros => {
            config.allow_all_macros = parse_bool_or_true(value)?;
        }
    }

    Ok(())
}

fn parse_bool_or_true(value: Option<&Expr>) -> syn::Result<bool> {
    match value {
        None => Ok(true),
        Some(s) => parse_bool(s),
    }
}

enum Var {
    EmitOriginal,
    EmitInert,
    AllowAllMacros,
}

fn parse_var(v: &ExprPath) -> syn::Result<Var> {
    if !v.attrs.is_empty() || v.qself.is_some() {
        return Err(syn::Error::new(
            v.span(),
            format!("unsupported attribute. {HELP}"),
        ));
    }

    let Some(v) = v.path.get_ident() else {
        return Err(syn::Error::new(
            v.span(),
            format!("unsupported attribute. {HELP}"),
        ));
    };

    match v.to_string().as_str() {
        "emit_original" => Ok(Var::EmitOriginal),
        "emit_inert" => Ok(Var::EmitInert),
        "allow_all_macros" => Ok(Var::AllowAllMacros),
        _ => Err(syn::Error::new(
            v.span(),
            format!("unsupported attribute. {HELP}"),
        )),
    }
}

fn parse_bool(v: &Expr) -> syn::Result<bool> {
    match parse_lit(v)? {
        Lit::Bool(bool) => Ok(bool.value),
        _ => Err(syn::Error::new(
            v.span(),
            format!("unsupported attribute… expected bool. {HELP}"),
        )),
    }
}

fn parse_string(v: &Expr) -> syn::Result<String> {
    match parse_lit(v)? {
        Lit::Str(s) => Ok(s.value()),
        _ => Err(syn::Error::new(
            v.span(),
            format!("unsupported attribute… expected string. {HELP}"),
        )),
    }
}

fn parse_lit(v: &Expr) -> syn::Result<Lit> {
    match v {
        Expr::Lit(l) => {
            if !l.attrs.is_empty() {
                return Err(syn::Error::new(
                    v.span(),
                    format!("unsupported attribute. {HELP}"),
                ));
            }
            Ok(l.lit.clone())
        }
        _ => Err(syn::Error::new(
            v.span(),
            format!("unsupported attribute. {HELP})),"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn codegen_requires_an_explicit_destruction_policy() {
        let error = CodegenConfig::from_attributes(TokenStream::new())
            .err()
            .expect("an omitted destruction policy must be rejected");
        assert!(error.to_string().contains("requires `drop_timing"));
    }

    #[test]
    fn codegen_accepts_the_explicit_relaxed_policy() {
        CodegenConfig::from_attributes(quote!(drop_timing = "relaxed")).unwrap();
    }

    #[test]
    fn codegen_rejects_unknown_drop_timing() {
        let error = CodegenConfig::from_attributes(quote!(drop_timing = "eventually"))
            .err()
            .expect("an unknown timing policy must be rejected");
        assert!(
            error
                .to_string()
                .contains("only supported destruction policy")
        );
    }
}
