use crate::{Fold, Result, context::FoldContext, ir::Folded};
use colorless_lower::ir::{CpsBody, CpsStmt, CpsStmtDirective, CpsTerminator};
pub mod block;
pub mod borrow;
pub mod branch;
pub mod cps;
pub mod loop_;
pub mod scoped;

impl Fold for CpsBody {
    type Output = Folded;

    fn fold(self, _below: (), ctx: &mut FoldContext) -> Result<Self::Output> {
        // Compose the body from its terminator upward.
        let below = match self.terminator {
            None => Folded::hole(),
            Some(terminator) => terminator.fold((), ctx)?,
        };

        self.stmts
            .into_iter()
            .rev()
            .try_fold(below, |below, stmt| stmt.fold(below, ctx))
    }
}

impl Fold<Folded> for CpsStmt {
    type Output = Folded;

    fn fold(self, below: Folded, ctx: &mut FoldContext) -> Result<Self::Output> {
        match self {
            CpsStmt::Directive(CpsStmtDirective::LocalPin(l)) => l.fold(below, ctx),
            CpsStmt::Directive(CpsStmtDirective::LocalPinEnd(le)) => le.fold(below, ctx),
            CpsStmt::Resolve(resolve) => resolve.fold(below, ctx),
            CpsStmt::Plain(plain) => plain.fold(below, ctx),
        }
    }
}

impl Fold for CpsTerminator {
    type Output = Folded;

    fn fold(self, (): (), ctx: &mut FoldContext) -> Result<Self::Output> {
        match self {
            CpsTerminator::ContUse(cont) => cont.fold((), ctx),
            CpsTerminator::Scoped(scoped) => scoped.fold((), ctx),
            CpsTerminator::Signal(cps_signal) => cps_signal.fold((), ctx),
        }
    }
}
