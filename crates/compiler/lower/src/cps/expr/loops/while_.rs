//! CPS lowering for `while cond { ... }`.
use crate::{
    cps::expr::CpsBind,
    cps::{CpsContinuation, CpsCtx, CpsStep},
    util::syn::{make_syn_block, make_syn_break, make_syn_if_else},
};
use syn::Ident;
use syn::token::Loop;
use syn::{ExprLoop, ExprWhile};

impl CpsStep for ExprWhile {
    fn then(
        self,
        continuation: CpsContinuation,
        ctx: &mut CpsCtx,
    ) -> crate::Result<CpsContinuation> {
        desugar_while(self).then(continuation, ctx)
    }
}

impl CpsBind for ExprWhile {
    fn and_then(
        self,
        bind: Ident,
        continuation: CpsContinuation,
        ctx: &mut CpsCtx,
    ) -> crate::Result<CpsContinuation> {
        desugar_while(self).and_then(bind, continuation, ctx)
    }
}

fn desugar_while(w: ExprWhile) -> ExprLoop {
    let if_expr = make_syn_if_else(w.cond, w.body, make_syn_break(None));

    ExprLoop {
        attrs: w.attrs,
        label: w.label,
        loop_token: Loop(w.while_token.span),
        body: make_syn_block(if_expr),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;
    use syn::{ExprLoop, ExprWhile, parse_quote};

    #[test]
    fn desugar_unlabeled_while_loop() {
        let expr: ExprWhile = parse_quote! {
            while cond {
                step();
            }
        };
        let desugared = desugar_while(expr);
        let expected: ExprLoop = parse_quote! {
            loop {
                if cond {
                    step();
                } else {
                    break
                }
            }
        };
        assert_eq!(
            quote!(#desugared).to_string(),
            quote!(#expected).to_string()
        );
    }

    #[test]
    fn desugar_labeled_while_loop() {
        let expr: ExprWhile = parse_quote! {
            'my_while: while cond {
                step();
            }
        };
        let desugared = desugar_while(expr);
        let expected: ExprLoop = parse_quote! {
            'my_while: loop {
                if cond {
                    step();
                } else {
                    break
                }
            }
        };
        assert_eq!(
            quote!(#desugared).to_string(),
            quote!(#expected).to_string()
        );
    }

    #[test]
    fn while_with_attrs_preserved_in_desugaring() {
        let expr: ExprWhile = parse_quote! {
            #[inline]
            while cond {}
        };
        let desugared = desugar_while(expr);
        assert_eq!(desugared.attrs.len(), 1);
    }
}
