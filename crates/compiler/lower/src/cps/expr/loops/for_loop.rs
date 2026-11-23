//! CPS lowering for `for pat in iter { ... }`.

use crate::cps::directive::is_colorless_for_loop;
use crate::util::syn::{make_stmt_expr, make_syn_block_stmts, make_syn_call};
use crate::{
    cps::expr::CpsBind,
    cps::{CpsContinuation, CpsCtx, CpsStep},
    util::syn::{
        make_syn_arm, make_syn_block, make_syn_break, make_syn_expr_block, make_syn_match,
        make_syn_match_, make_syn_none_arm, make_syn_some_arm, mut_ident_pat,
    },
};
use syn::token::Loop;
use syn::{Expr, ExprForLoop, ExprLoop, ExprMatch};
use syn::{Ident, Stmt, parse_quote};

impl CpsStep for ExprForLoop {
    fn then(
        self,
        continuation: CpsContinuation,
        ctx: &mut CpsCtx,
    ) -> crate::Result<CpsContinuation> {
        lower_for_loop(self, ctx).then(continuation, ctx)
    }
}

impl CpsBind for ExprForLoop {
    fn and_then(
        self,
        bind: Ident,
        continuation: CpsContinuation,
        ctx: &mut CpsCtx,
    ) -> crate::Result<CpsContinuation> {
        lower_for_loop(self, ctx).and_then(bind, continuation, ctx)
    }
}
fn lower_for_loop(mut f: ExprForLoop, ctx: &mut CpsCtx) -> ExprMatch {
    let iter_placeholder = ctx.next_temp_ident("iter");

    if ctx.config.auto_inject_loop_state {
        crate::cps::directive::add_hidden_loop_binding(&mut f.attrs, &iter_placeholder);
    }

    desugar_for_loop_(f, iter_placeholder, ctx)
}

fn desugar_for_loop_(mut f: ExprForLoop, iter_placeholder: Ident, ctx: &mut CpsCtx) -> ExprMatch {
    let colorless = is_colorless_for_loop(&mut f.attrs);

    let iter_next: Expr = if colorless {
        parse_quote!(reborrow!(#iter_placeholder).next().resolve)
    } else {
        parse_quote!(#iter_placeholder.next())
    };

    let iter_none_arm = make_syn_none_arm(make_syn_break(None));
    let iter_some_arm = make_syn_some_arm(*f.pat, make_syn_expr_block(f.body));

    let iter_match_expr = make_syn_match(iter_next, [iter_none_arm, iter_some_arm]);

    let mut l = Expr::Loop(ExprLoop {
        attrs: f.attrs,
        label: f.label,
        loop_token: Loop(f.for_token.span),
        body: make_syn_block(iter_match_expr),
    });

    // TODO: Use linear_borrow_loop instead... add as ref_mut
    let convert = if colorless {
        let mut pin_borrow: Vec<Stmt> = parse_quote! {
            pin!(#iter_placeholder);
            borrow!(#iter_placeholder);
        };
        pin_borrow.push(make_stmt_expr(l));
        // FIXME...
        pin_borrow.push(parse_quote! {
            unborrow!(#iter_placeholder);
        });
        pin_borrow.push(parse_quote! {
            unpin!(#iter_placeholder);
        });

        l = make_syn_block_stmts(pin_borrow);

        ctx.config.colorless_into_iterator.clone()
    } else {
        parse_quote!(::core::iter::IntoIterator::into_iter)
    };
    let into_iter = make_syn_call(convert, [*f.expr]);

    let into_iter_arm = make_syn_arm(mut_ident_pat(iter_placeholder), l);
    make_syn_match_(into_iter, [into_iter_arm])
}

#[cfg(test)]
fn desugar_for_loop(f: ExprForLoop, iter_placeholder: Ident) -> ExprMatch {
    desugar_for_loop_(f, iter_placeholder, &mut CpsCtx::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::{format_ident, quote};
    use syn::{Expr, ExprForLoop, ExprMatch, parse_quote};

    #[test]
    fn desugar_unlabeled_for_loop() {
        let expr: ExprForLoop = parse_quote! {
            for x in items {
                process(x);
            }
        };
        let iter = format_ident!("__iter");
        let desugared = desugar_for_loop(expr, iter);

        let expected: ExprMatch = parse_quote! {
            match ::core::iter::IntoIterator::into_iter(items) {
                mut __iter => loop {
                    match __iter.next() {
                        None => break,
                        Some(x) => {
                            process(x);
                        },
                    }
                },
            }
        };
        assert_eq!(
            quote!(#desugared).to_string(),
            quote!(#expected).to_string()
        );
    }

    #[test]
    fn desugar_labeled_for_loop() {
        let expr: ExprForLoop = parse_quote! {
            'items_loop: for x in items {
                process(x);
            }
        };
        let iter = syn::Ident::new("__iter", proc_macro2::Span::call_site());
        let desugared = desugar_for_loop(expr, iter);
        let expected: ExprMatch = parse_quote! {
            match ::core::iter::IntoIterator::into_iter(items) {
                mut __iter => 'items_loop: loop {
                    match __iter.next() {
                        None => break,
                        Some(x) => {
                            process(x);
                        },
                    }
                },
            }
        };

        assert_eq!(
            quote!(#desugared).to_string(),
            quote!(#expected).to_string()
        );
    }

    #[test]
    fn for_with_attrs_preserved_in_desugaring() {
        let expr: ExprForLoop = parse_quote! {
            #[inline]
            for x in items {}
        };
        let iter = syn::Ident::new("__iter", proc_macro2::Span::call_site());
        let desugared = desugar_for_loop(expr, iter);
        let arm = &desugared.arms[0];
        if let Expr::Loop(l) = &*arm.body {
            assert_eq!(l.attrs.len(), 1);
        } else {
            panic!("expected ExprLoop");
        }
    }
}
