use proc_macro2::Span;
use syn::parse::{Parse, ParseStream};
use syn::{GenericParam, Generics, Ident, Path, parse_quote};
use syn::{Token, Type};

#[derive(Debug)]
pub struct Config {
    pub drop_timing: DropTiming,
    pub autocomplete: bool,
    pub color: Color,
    pub crate_path: Path,
}

#[derive(Debug)]
pub enum Color {
    Generic(Ident),
    Specific(Box<Type>),
    Auto,
}

#[derive(Clone, Debug)]
pub enum DropTiming {
    Relaxed,
    None,
}

impl Default for Config {
    fn default() -> Self {
        let (crate_path, drop_timing): (Path, DropTiming) =
            match std::env::var("CARGO_PKG_NAME").as_deref() {
                // We don't want colorless-std infecting others... we should prolly
                // bring back per-call annotation
                Ok("colorless-std") => (parse_quote!(::colorless_rt), DropTiming::Relaxed),
                _ => (
                    parse_quote!(::colorless),
                    if cfg!(feature = "relaxed-drop") {
                        DropTiming::Relaxed
                    } else {
                        DropTiming::None
                    },
                ),
            };
        Self {
            drop_timing,
            autocomplete: cfg!(feature = "autocomplete"),
            color: Color::Auto,
            crate_path,
        }
    }
}

impl Parse for Config {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut config = Config::default();

        if !matches!(config.drop_timing, DropTiming::Relaxed) {
            return Err(syn::Error::new(
                Span::call_site(),
                "colorless currently supports only relaxed implicit destruction timing; \
         transformed values may be destroyed earlier or later than their source scope and \
         implicit sibling destruction order may differ.\n\n\
         To acknowledge this, enable the `relaxed-drop` feature globally in your Cargo.toml:\n    \
           colorless = { version = \"...\", features = [\"relaxed-drop\"] }",
            ));
        }

        if !input.is_empty() {
            if let Some(first) = input.parse()? {
                if first == "color" {
                    input.parse::<Token![=]>()?;
                    config.color = Color::Specific(input.parse()?);
                } else {
                    config.color = Color::Generic(first);
                }
            } else {
                return Err(syn::Error::new(
                    input.span(),
                    "expected `#[colorless(color = Async)]`, or `#[colorless(C)]`",
                ));
            }

            if !input.is_empty() {
                return Err(syn::Error::new(
                    input.span(),
                    "`#[colorless]` cannot take more than one argument",
                ));
            }
        }

        Ok(config)
    }
}

impl Color {
    pub fn get_or_create_generic_color(&self, generics: &Generics) -> Result<Ident, syn::Error> {
        self.try_get_or_create_generic_color(generics).map_err(|s| {
            syn::Error::new_spanned(
                s,
                "`#[colorless]` does not take color argument in this position.",
            )
        })
    }

    pub fn try_get_or_create_generic_color(&self, generics: &Generics) -> Result<Ident, &Type> {
        match self {
            Color::Generic(g) => Ok(g.clone()),
            Color::Specific(s) => Err(s),
            Color::Auto => Ok(fresh_color_ident(generics)),
        }
    }
}

pub fn fresh_color_ident(generics: &Generics) -> Ident {
    let is_used = |candidate: &str| {
        generics.params.iter().any(|parameter| match parameter {
            GenericParam::Lifetime(parameter) => parameter.lifetime.ident == candidate,
            GenericParam::Type(parameter) => parameter.ident == candidate,
            GenericParam::Const(parameter) => parameter.ident == candidate,
        })
    };

    const COLOR_IDENT: &str = "__C";

    if !is_used(COLOR_IDENT) {
        return Ident::new(COLOR_IDENT, Span::call_site());
    }

    let mut suffix = 1usize;
    loop {
        let candidate = format!("{COLOR_IDENT}{suffix}");
        if !is_used(&candidate) {
            return Ident::new(&candidate, Span::call_site());
        }
        suffix += 1;
    }
}
