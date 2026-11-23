use crate::Fold;
use crate::context::FoldContext;
use crate::ir::{Folded, FoldedOutput, YieldPoint};
use crate::runtime_call::call_once_move;
use crate::util::{bind_or_discard, stmts};
use colorless_lower::ir::{CpsPlainStmt, CpsStmtResolve, IntoSyn};
use syn::{Expr, Pat, Result};

impl Fold<Folded> for CpsStmtResolve {
    type Output = Folded;

    fn fold(self, below: Folded, ctx: &mut FoldContext) -> Result<Self::Output> {
        let first = (*self.base).into_syn();
        let converted = if ctx.config.desugar_resolve {
            ctx.call_into_colored(first)
        } else {
            first
        };

        if self.semi_token.is_none() {
            assert!(below.is_hole());
            assert!(self.bind.is_none());
            return Ok(Folded {
                output: FoldedOutput {
                    yield_point: YieldPoint::Colored,
                },
                stmts: stmts(converted),
            });
        }

        Ok(fold_colored(
            ctx,
            converted,
            bind_or_discard(self.bind),
            below,
        ))
    }
}

impl Fold<Folded> for CpsPlainStmt {
    type Output = Folded;

    fn fold(self, mut below: Folded, _ctx: &mut FoldContext) -> crate::Result<Self::Output> {
        // plain preserves the output
        below.stmts.push_front(self.into_syn());
        Ok(below)
    }
}

pub fn fold_colored(ctx: &FoldContext, first: Expr, bind: Pat, below: Folded) -> Folded {
    let next = call_once_move(bind, below.stmts);
    fold_all_colored_with(ctx, first, next, below.output)
}

pub fn fold_all_colored_with(
    ctx: &FoldContext,
    first: Expr,
    closure: Expr,
    mut closure_output: FoldedOutput,
) -> Folded {
    let new = fold_all_colored_with_(ctx, first, closure, &mut closure_output);
    Folded {
        output: closure_output,
        stmts: stmts(new),
    }
}

pub fn fold_all_colored_with_(
    ctx: &FoldContext,
    first: Expr,
    closure: Expr,
    closure_output: &mut FoldedOutput,
) -> Expr {
    let cps_method = if closure_output.requires_color() {
        "and_then"
    } else {
        "map"
    };

    let new = ctx.call("Cps", cps_method, [first, closure]);

    closure_output.yield_point = YieldPoint::Colored;
    new
}
