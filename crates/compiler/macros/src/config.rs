use crate::util::add_generic;
use proc_macro2::Span;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{GenericParam, Generics, Ident, Path, Token, Type, parse_quote, token};

/// User-specified extension to the macro allowlist in `#[colorless]`.
#[derive(Debug, Default)]
pub enum AllowMacros {
    /// Keep the built-in default allowlist (unchanged).
    #[default]
    Default,
    /// Permit every macro invocation inside this item.
    All,
    /// Augment the built-in list with explicit extra paths.
    Custom(Vec<Path>),
}

#[derive(Debug)]
pub struct Config {
    pub drop_timing: DropTiming,
    pub autocomplete: bool,
    pub desugar_resolve: bool,
    pub color: Color,
    pub crate_path: Path,
    pub wrap_policy: WrapPolicy,
    pub erase: bool,
    /// Macro invocations to permit beyond the built-in allowlist.
    pub allow_macros: AllowMacros,
    pub dev_mode: bool,
}

#[derive(Debug)]
pub enum Color {
    Generic(Ident),
    Specific(Box<Type>),
    DeclaredGeneric(Ident),
    Auto,
}

#[derive(Clone, Debug)]
pub enum DropTiming {
    Relaxed,
    None,
}

#[derive(Debug)]
pub enum WrapPolicy {
    NoWrap,
    Wrap,
    Unspecified,
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
            desugar_resolve: cfg!(feature = "desugar-resolve"),
            color: Color::Auto,
            crate_path,
            wrap_policy: WrapPolicy::Unspecified,
            erase: false,
            allow_macros: AllowMacros::Default,
            dev_mode: false,
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

        // Parse a comma-separated list of arguments:
        //   #[colorless]
        //   #[colorless(C)]
        //   #[colorless(color = Async)]
        //   #[colorless(allow_macros(my_macro, some_crate::mac))]
        //   #[colorless(allow_macros(all))]
        //   #[colorless(C, allow_macros(my_macro))]
        //   #[colorless(color = Async, allow_macros(all))]
        let mut color_set = false;
        while !input.is_empty() {
            // Peek whether this token is `allow_macros`
            let lookahead = input.lookahead1();
            if lookahead.peek(Ident) {
                let ident: Ident = input.fork().parse()?;
                match ident.to_string().as_str() {
                    "allow_macros" => {
                        // consume it
                        let _: Ident = input.parse()?;
                        // must be followed by parentheses
                        let content;
                        let _paren: token::Paren = syn::parenthesized!(content in input);
                        config.allow_macros = parse_allow_macros_args(&content)?;
                    }
                    "color" => {
                        if color_set {
                            return Err(syn::Error::new(
                                input.span(),
                                "`#[colorless]` cannot take more than one color argument",
                            ));
                        }
                        let _: Ident = input.parse()?;
                        input.parse::<Token![=]>()?;
                        config.color = Color::Specific(input.parse()?);
                        color_set = true;
                    }
                    "no_wrap" => {
                        if !matches!(config.wrap_policy, WrapPolicy::Unspecified) {
                            return Err(syn::Error::new(
                                input.span(),
                                "conflicting/duplicated wrap policy",
                            ));
                        }
                        let _: Ident = input.parse()?;

                        config.wrap_policy = WrapPolicy::NoWrap;
                    }
                    "wrap" => {
                        if !matches!(config.wrap_policy, WrapPolicy::Unspecified) {
                            return Err(syn::Error::new(
                                input.span(),
                                "conflicting/duplicated wrap policy",
                            ));
                        }
                        let _: Ident = input.parse()?;

                        config.wrap_policy = WrapPolicy::Wrap;
                    }
                    "erase" => {
                        if config.erase {
                            return Err(syn::Error::new(input.span(), "redundant `erase`"));
                        }
                        let _: Ident = input.parse()?;
                        config.erase = true;
                    }
                    "out_of_my_way" | "dev_mode" | "arrgh" => {
                        if config.dev_mode {
                            return Err(syn::Error::new(input.span(), "redundant `dev-mode`"));
                        }
                        let _: Ident = input.parse()?;
                        config.dev_mode = true;
                    }
                    _ => {
                        // color generic ident, optionally `C as`
                        if color_set {
                            return Err(syn::Error::new(
                                input.span(),
                                "`#[colorless]` cannot take more than one color argument",
                            ));
                        }
                        let name: Ident = input.parse()?;
                        let as_: Option<Token![as]> = input.parse()?;
                        if as_.is_some() {
                            config.color = Color::DeclaredGeneric(name);
                        } else {
                            config.color = Color::Generic(name);
                        }
                        color_set = true;
                    }
                }
            } else {
                return Err(syn::Error::new(
                    input.span(),
                    "expected `color = ...`, a color generic ident, or `allow_macros(...)`",
                ));
            }

            // consume optional comma between arguments
            if input.peek(Token![,]) {
                let _: Token![,] = input.parse()?;
            }
        }

        Ok(config)
    }
}

/// Parse the argument list inside `allow_macros(...)`.
///
/// Accepts either `all` (bare ident) or a comma-separated list of paths:
/// `allow_macros(all)` → `AllowMacros::All`
/// `allow_macros(my_mac, crate::other)` → `AllowMacros::Custom([...])`
fn parse_allow_macros_args(content: ParseStream) -> syn::Result<AllowMacros> {
    // Single keyword `all`
    if content.peek(Ident) {
        let fork = content.fork();
        let ident: Ident = fork.parse()?;
        if ident == "all" && fork.is_empty() {
            // consume from real stream
            let _: Ident = content.parse()?;
            return Ok(AllowMacros::All);
        }
    }

    // Otherwise parse as comma-separated paths
    let paths = Punctuated::<Path, Token![,]>::parse_terminated(content)?;
    if paths.is_empty() {
        return Err(syn::Error::new(
            content.span(),
            "`allow_macros()` requires at least one macro path, or `all`",
        ));
    }
    Ok(AllowMacros::Custom(paths.into_iter().collect()))
}

impl Config {
    pub fn or_add_generic(&mut self, generics: &mut Generics, default: bool) {
        match self.color {
            Color::Generic(ref g) => {
                add_generic(generics, &self.crate_path, g, default, self.erase);
            }
            Color::Specific(ref _s) => {}
            Color::DeclaredGeneric(ref _dg) => {}
            Color::Auto => {
                let f = fresh_color_ident(generics);
                add_generic(generics, &self.crate_path, &f, default, self.erase);
                self.color = Color::DeclaredGeneric(f);
            }
        }
    }

    pub fn unwrap_color(&self) -> Result<&Ident, &Type> {
        match self.color {
            Color::Generic(ref g) => Ok(g),
            Color::Specific(ref s) => Err(s),
            Color::DeclaredGeneric(ref dg) => Ok(dg),
            _ => {
                panic!("internal error: color should be expanded at this point")
            }
        }
    }

    pub fn unwrap_generic_color(&self) -> Result<&Ident, syn::Error> {
        self.unwrap_color().map_err(|s| {
            syn::Error::new_spanned(
                s,
                "`#[colorless]` does not take color argument in this position.",
            )
        })
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
