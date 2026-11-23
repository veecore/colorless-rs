use crate::{
    Fold, Result,
    context::FoldContext,
    ir::{Folded, FoldedOutput},
    runtime_call::call_once_move,
    util::ident_expr,
    util::{ident_pat, unterminated_stmt},
};
use colorless_lower::ir::{CpsStmtDirectiveLocalPin, CpsStmtDirectiveLocalPinEnd};
use std::collections::VecDeque;

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
            return Err(syn::Error::new(
                self.input.span,
                "Redundant pin; pin is only required when the reference crosses resolve point",
            ));
        }

        // I'd love to lock ctx here to avoid drift
        let unpin_encountered = ctx.unpin_encountered;

        let arg = if unpin_encountered {
            ctx.wrap_in_maybe_unpinned(local)
        } else {
            ident_expr(local.clone())
        };

        let pat = ident_pat(local.clone());

        let mut output = VecDeque::with_capacity(self.input.locals.len());

        if !unpin_encountered {
            below.stmts.push_front(ctx.call_erased_into_mut(local))
        }

        let type_store = ctx.fresh_ident("type");
        below.stmts.push_front(if unpin_encountered {
            ctx.call_retrieve_type(local, &type_store)
        } else {
            ctx.call_retrieve_type_unpinned(local, &type_store)
        });

        output.push_back(ctx.call_store_type(local, &type_store));

        let borrow = ctx.call(
            "LocalBorrow",
            "borrow_flat",
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

    fn fold(self, mut below: Folded, ctx: &mut FoldContext) -> Result<Self::Output> {
        ctx.unpin_encountered = true;

        self.input
            .locals
            .into_iter()
            .for_each(|x| below.stmts.push_front(ctx.call_unpin(&x.ident)));

        Ok(below)
    }
}
