//! Parsers for Colorless compiler directives consumed during CPS lowering.
//!
//! Directives (`#[colorless::loop_state(...)]`, `pin_local!(...)`, `unpin_local!(...)`)
//! are recognized at this boundary and stripped before emitting CPS IR.

use crate::ir::CpsLocalPinInput;
use crate::{
    CpsError, Result,
    ir::{CpsLocalPinTarget, CpsLoopBinding, CpsLoopHint, LoopStateInput},
};
use std::collections::{BTreeSet, HashSet};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::{Attribute, Ident, Macro, Token, parse_quote};

const PIN_LOCAL: &str = "pin_local";
const UNPIN_LOCAL: &str = "unpin_local";
const LOOP_STATE: &str = "loop_state";
const NO_SHADOW: &str = "inherit_no_shadow";
const DEFAULT_BREAK_STATE: &str = "default_break_state";
const COLORLESS: &str = "colorless";

pub(crate) enum PinBorrowDirective {
    LocalStart(CpsLocalPinInput),
    LocalEnd(CpsLocalPinInput),
}

/// Parse a statement macro if it is one of Colorless's local-borrow markers.
/// Returning `None` delegates the macro to the ordinary allowlist.
pub(crate) fn parse_borrow_directive(mac: &Macro) -> Result<Option<PinBorrowDirective>> {
    if path_ends_with(&mac.path, PIN_LOCAL) {
        let mut input: CpsLocalPinInput = syn::parse2(mac.tokens.clone()).map_err(|e| {
            CpsError::Other(syn::Error::new_spanned(
                mac,
                format!("`pin_local!` argument is invalid: {e}"),
            ))
        })?;
        // red on macro is more obvious than one ident
        input.span = mac.span();

        return Ok(Some(PinBorrowDirective::LocalStart(input)));
    }

    if path_ends_with(&mac.path, UNPIN_LOCAL) {
        let mut input: CpsLocalPinInput = syn::parse2(mac.tokens.clone()).map_err(|e| {
            CpsError::Other(syn::Error::new_spanned(
                mac,
                format!("`unpin_local!` argument is invalid: {e}"),
            ))
        })?;
        input.span = mac.span();
        return Ok(Some(PinBorrowDirective::LocalEnd(input)));
    }

    Ok(None)
}

impl Parse for CpsLocalPinInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let span = input.span();
        let locals = input.parse_terminated(Ident::parse, Comma)?;
        let len = locals.len();

        let locals: HashSet<_> = locals
            .into_iter()
            .map(|ident| CpsLocalPinTarget { ident })
            .collect();

        if locals.len() != len {
            return Err(syn::Error::new(
                input.span(),
                "a local-pin binding may be listed only once",
            ));
        }
        Ok(CpsLocalPinInput { span, locals })
    }
}

/// Add state owned by a source desugaring to the same directive consumed by
/// ordinary loop lowering.
///
/// A `for` loop's iterator is bound outside its generated `loop`, so it must be
/// threaded through a lazy loop step just like user-declared state. Keeping the
/// binding on the generated loop lets `for` continue through the canonical
/// `match`/`loop` lowering path. The attribute is consumed immediately when
/// that loop is lowered and never reaches the CPS IR.
pub(crate) fn add_hidden_loop_binding(attrs: &mut Vec<Attribute>, ident: &Ident) {
    attrs.push(parse_quote!(#[colorless::loop_state(mut #ident)]))
}

/// Consume the single loop-state directive, leaving ordinary attributes for
/// the caller's usual rejection policy.
pub(crate) fn take_loop_state(attrs: Vec<Attribute>) -> Result<(CpsLoopHint, Vec<Attribute>)> {
    let mut state_bindings = BTreeSet::new();
    let mut remaining = Vec::with_capacity(attrs.len());

    let mut no_shadow = false;
    let mut default_break_state = false;

    for attr in attrs {
        if path_ends_with(attr.path(), NO_SHADOW) {
            if no_shadow {
                return Err(CpsError::Other(syn::Error::new_spanned(
                    attr,
                    "a loop may only be marked `#[inherit_no_shadow]` once.",
                ))
                .into());
            }
            no_shadow = true;
        } else if path_ends_with(attr.path(), LOOP_STATE) {
            let parsed = attr
                .parse_args::<LoopStateInput>()
                .map_err(CpsError::Other)?;

            state_bindings.extend(parsed.bindings);
        } else if path_ends_with(attr.path(), DEFAULT_BREAK_STATE) {
            default_break_state = true;
        } else {
            remaining.push(attr);
        }
    }

    let state = CpsLoopHint {
        no_shadow,
        default_break_state,
        state: LoopStateInput {
            bindings: state_bindings,
        },
    };

    Ok((state, remaining))
}

pub(crate) fn is_colorless_for_loop(attrs: &mut Vec<Attribute>) -> bool {
    let l = attrs.len();
    attrs.retain(|attr| !path_ends_with(attr.path(), COLORLESS));

    l != attrs.len()
}

impl Parse for LoopStateInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut bindings = BTreeSet::new();
        while !input.is_empty() {
            let thread = input.parse::<Option<Token![pub]>>()?;
            let referenced = input.parse::<Option<Token![ref]>>()?;
            let mutability = input.parse::<Option<Token![mut]>>()?;
            // todo: self is not ident... handle that
            let ident = input.parse::<Ident>()?;
            let span = ident.span();

            if !bindings.insert(CpsLoopBinding {
                referenced,
                mutability,
                thread: thread.is_some(),
                ident,
            }) {
                return Err(syn::Error::new(
                    span,
                    "a loop-state binding may be listed only once",
                ));
            }

            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }

        Ok(Self { bindings })
    }
}

fn path_ends_with(path: &syn::Path, name: &str) -> bool {
    match path.segments.len() {
        1 => path.segments[0].ident == name,
        2 => path.segments[0].ident == COLORLESS && path.segments[1].ident == name,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use crate::CpsError;
    use crate::cps::CpsStep;
    use crate::cps::Ident;
    use crate::cps::directive::BTreeSet;
    use crate::cps::directive::PinBorrowDirective;
    use crate::cps::directive::add_hidden_loop_binding;
    use crate::cps::directive::parse_borrow_directive;
    use crate::cps::directive::take_loop_state;
    use crate::ir::CpsLocalPinInput;
    use crate::ir::CpsLocalPinTarget;
    use crate::ir::CpsLoopBinding;
    use crate::ir::scoped::{CpsScoped, CpsTerminatorScoped};
    use crate::ir::{CpsBody, CpsTerminator};
    use proc_macro2::Span;
    use quote::format_ident;
    use syn::{ExprLoop, Stmt, parse_quote};

    fn binding(s: &str) -> CpsLoopBinding {
        CpsLoopBinding {
            referenced: None,
            mutability: None,
            thread: false,
            ident: Ident::new(s, Span::call_site()),
        }
    }
    fn binding_mut(s: &str) -> CpsLoopBinding {
        CpsLoopBinding {
            referenced: None,
            mutability: Some(Default::default()),
            thread: false,
            ident: Ident::new(s, Span::call_site()),
        }
    }
    #[test]
    fn parses_owned_loop_state_without_retaining_attribute_syntax() {
        let loop_: ExprLoop = parse_quote!(
            #[colorless::loop_state(mut state, index)]
            loop {}
        );
        let (state, attrs) = take_loop_state(loop_.attrs).unwrap();
        assert!(attrs.is_empty());

        assert_eq!(
            BTreeSet::from([binding_mut("state"), binding("index")]),
            state.state.bindings
        );
    }

    #[test]
    fn hidden_loop_binding_extends_owned_directives() {
        let hidden: Ident = parse_quote!(__iter);
        let mut owned: ExprLoop = parse_quote!(
            #[colorless::loop_state(mut state)]
            loop {}
        );
        add_hidden_loop_binding(&mut owned.attrs, &hidden);
        let (owned, remaining) = take_loop_state(owned.attrs).unwrap();
        assert!(remaining.is_empty());

        assert_eq!(
            BTreeSet::from([binding_mut("state"), binding("__iter")]),
            owned.state.bindings
        );
    }

    #[test]
    fn parses_borrow_markers_into_typed_nodes() {
        let Stmt::Macro(start) = parse_quote!(colorless::pin_local!(local);) else {
            panic!("expected statement macro")
        };
        let Stmt::Macro(end) = parse_quote!(colorless::unpin_local!(local);) else {
            panic!("expected statement macro")
        };

        assert!(matches!(
            parse_borrow_directive(&start.mac).unwrap(),
            Some(PinBorrowDirective::LocalStart(CpsLocalPinInput {locals, ..})) if locals.contains(&CpsLocalPinTarget {
                    ident: format_ident!("local"),
                }) && locals.len() == 1,
        ));
        assert!(matches!(
            parse_borrow_directive(&end.mac).unwrap(),
            Some(PinBorrowDirective::LocalEnd(CpsLocalPinInput {locals, ..})) if locals.contains(&CpsLocalPinTarget {
                    ident: format_ident!("local"),
                }) && locals.len() == 1,
        ));
    }

    #[test]
    fn lowering_attaches_state_to_the_loop_node() {
        let loop_: ExprLoop = parse_quote!(
            #[colorless::loop_state(mut state, index)]
            loop {}
        );
        let body = CpsBody::from(loop_.step().unwrap());
        let Some(CpsTerminator::Scoped(CpsTerminatorScoped {
            scoped: CpsScoped::Loop(loop_),
            ..
        })) = body.terminator.as_deref()
        else {
            panic!("expected structural loop terminator")
        };

        assert_eq!(
            BTreeSet::from([binding_mut("state"), binding("index")]),
            loop_.cps_loop_metadata.hint.state.bindings
        );
    }

    #[test]
    fn duplicate_loop_state_directives_are_accepted() {
        let loop_: ExprLoop = parse_quote!(
            #[colorless::loop_state(state)]
            #[colorless::loop_state(other)]
            loop {}
        );
        let body = loop_.step().unwrap();
        let Some(CpsTerminator::Scoped(CpsTerminatorScoped {
            scoped: CpsScoped::Loop(loop_),
            ..
        })) = body.terminator.as_deref()
        else {
            panic!("expected structural loop terminator")
        };

        assert_eq!(
            BTreeSet::from([binding_mut("state"), binding("other")]),
            loop_.cps_loop_metadata.hint.state.bindings
        );
    }

    #[test]
    fn duplicate_state_bindings_are_rejected() {
        let loop_: ExprLoop = parse_quote!(
            #[colorless::loop_state(mut state, state)]
            loop {}
        );
        let error = loop_.step().unwrap_err();
        assert!(matches!(*error, CpsError::Other(_)));
    }
}
