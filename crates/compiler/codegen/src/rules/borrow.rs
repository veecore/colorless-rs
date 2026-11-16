use crate::{
    Fold, Result,
    context::FoldContext,
    ir::{Folded, FoldedOutput},
    runtime_call::call_once_move,
    util::{ident_pat, unterminated_stmt},
};
use colorless_lower::ir::{CpsStmtDirectiveLocalPin, CpsStmtDirectiveLocalPinEnd};
use quote::format_ident;
use std::collections::VecDeque;
use syn::{Stmt, parse_quote};

impl Fold<Folded> for CpsStmtDirectiveLocalPin {
    type Output = Folded;

    fn fold(self, mut below: Folded, ctx: &mut FoldContext) -> Result<Self::Output> {
        // TODO: WE NEED BATCHED PIN... ALTHOUGH WE CAN'T SHARE TAG, WE CAN REDUCE THE GROWTH
        //  OF ASYNC

        if self.input.locals.len() != 1 {
            return Err(syn::Error::new(
                self.input.span,
                "pin local can currently be only one",
            ));
        }

        let local = self.input.locals.iter().next().unwrap();

        let local = &local.ident;

        if !below.output.requires_color() {
            return if !ctx.config.allow_redundant_pin {
                Err(syn::Error::new(
                    self.input.span,
                    "Redundant pin; pin is only required when the reference crosses resolve point",
                ))
            } else {
                let unpin_macro = format_ident!("__unpin_{}", local);
                below.stmts.push_front(parse_quote! {
                    macro_rules! #unpin_macro {
                        () => {
                            #local
                        };
                    };

                    let mut #local = &mut #local;
                });
                Ok(below)
            };
        }

        let arg = ctx.wrap_in_maybe_unpinned(local);

        let pat = ident_pat(local.clone());

        let mut output = VecDeque::with_capacity(self.input.locals.len());

        below.stmts.push_front(ctx.call_erased_as_mut(local));

        // We can't use random id... the ambiguity with shadow doesn't seem avoidable
        //
        // let a = V;
        // pin_local!(a)
        // let a = R;
        // pin_local!(a)
        //
        // unpin_local!(a);
        // unpin_local!(a):
        //
        // it's actually just going to pass as double unpin that'd fail at compile-time
        // I now think we should remove this, the inert code targets the in-scope var
        // and we should too....
        // let a = V;
        // pin_local!(a);
        // let a = R;
        // // not pinned
        // unpin_local!(a); // this unpins V.... while this seems cool to me, a drift like this
        // between inert and might not be good
        let unpin_macro = format_ident!("__unpin_{}", local);
        let unpin = ctx.call_unpin(local);
        below.stmts.push_front(parse_quote! {
            macro_rules! #unpin_macro {
                () => {
                    #unpin
                };
            }
        });

        let type_store = ctx.fresh_ident("type");
        below
            .stmts
            .push_front(ctx.call_retrieve_type(local, &type_store));

        output.push_back(ctx.call_store_type(local, &type_store));

        let borrow = ctx.call(
            "LocalBorrow",
            "borrow",
            [
                arg,
                ctx.wrap_in_unsafe_fn_scope(call_once_move(pat, below.stmts)),
            ],
        );

        output.push_back(unterminated_stmt(borrow));

        Ok(Folded {
            output: FoldedOutput::colored(),
            stmts: output,
        })
    }
}

impl Fold<Folded> for CpsStmtDirectiveLocalPinEnd {
    type Output = Folded;

    fn fold(self, mut below: Folded, _ctx: &mut FoldContext) -> Result<Self::Output> {
        self.input.locals.into_iter().for_each(|x| {
            let local = &x.ident;
            let unpin_macro = format_ident!("__unpin_{}", local);

            below.stmts.push_front(parse_quote! {
                let mut #local = #unpin_macro!();
            });
        });
        Ok(below)
    }
}

pub fn declare_borrow_macros(ctx: &FoldContext) -> Stmt {
    let krate = &ctx.config.crate_path;
    // for inert code, we just emit a different use
    parse_quote! {
        use #krate::{reborrow, borrow_mut, end_borrow_mut, keep_alive};
    }
}
