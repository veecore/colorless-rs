//! CPS transformation for sequences of statements, folding right-to-left.

use crate::ir::{CpsStmtDirective, CpsStmtDirectiveLocalPin, CpsStmtDirectiveLocalPinEnd};
use crate::{
    CpsError, Result,
    cps::CpsContinuation,
    cps::CpsCtx,
    cps::CpsStep,
    cps::anf::HoistTarget,
    cps::directive::{PinBorrowDirective, parse_borrow_directive},
    cps::expr::DivergingExpr,
    cps::expr::macro_::assert_valid_macro,
    error::assert_no_attrs,
    ir::scoped::CpsTerminatorScoped,
    ir::{CpsBlock, CpsBody, CpsStmtLocal, CpsTerminator},
    ir::{CpsLocalInit, CpsStmt, CpsStmtMacro},
    util::cps::unit_expr,
    util::cps::{make_arm, make_match},
    util::syn::wildcard_pat,
};
use syn::token::Semi;
use syn::{Expr, Item, Local, Stmt, StmtMacro};
// ---------------------------------------------------------------------------
// cps_flatten
// ---------------------------------------------------------------------------

/// Lower a sequence of statements into a flat [`CpsContinuation`], folding right-to-left
/// so that each statement's continuation is the already-lowered tail.
pub fn cps_transform(
    stmts: impl DoubleEndedIterator<Item = Stmt>,
    ctx: &mut CpsCtx,
) -> Result<CpsBlock> {
    fold_stmts(stmts, ctx).map(|continuation| continuation.into())
}

/// Lower a sequence of statements into a flat [`CpsContinuation`], folding right-to-left
/// so that each statement's continuation is the already-lowered tail.
#[inline]
pub fn fold_stmts(
    stmts: impl DoubleEndedIterator<Item = Stmt>,
    ctx: &mut CpsCtx,
) -> Result<CpsContinuation> {
    let buf = CpsContinuation::with_capacity(stmts.size_hint().0);
    fold_stmts_with_seed(stmts, buf, ctx)
}

/// Fold `stmts` right-to-left, each one consuming `seed` as its continuation.
///
/// `seed` is the continuation produced by the tail expression (or a unit binding
/// when there is no tail).
#[inline]
pub fn fold_stmts_with_seed(
    stmts: impl DoubleEndedIterator<Item = Stmt>,
    seed: CpsContinuation,
    ctx: &mut CpsCtx,
) -> Result<CpsContinuation> {
    stmts.rev().try_fold(seed, |acc, stmt| stmt.then(acc, ctx))
}

// ---------------------------------------------------------------------------
// Vec<Stmt>
// ---------------------------------------------------------------------------

impl CpsStep for Vec<Stmt> {
    #[inline]
    fn then(self, c: CpsContinuation, ctx: &mut CpsCtx) -> Result<CpsContinuation> {
        fold_stmts_with_seed(self.into_iter(), c, ctx)
    }
}

impl CpsStep<CpsBody> for Vec<Stmt> {
    #[inline]
    fn then(self, c: CpsContinuation, ctx: &mut CpsCtx) -> Result<CpsBody> {
        fold_stmts_with_seed(self.into_iter(), c, ctx).map(CpsBody::from)
    }
}

// ---------------------------------------------------------------------------
// Dispatch over Stmt
// ---------------------------------------------------------------------------

impl CpsStep for Stmt {
    #[inline]
    fn then(self, continuation: CpsContinuation, ctx: &mut CpsCtx) -> Result<CpsContinuation> {
        match self {
            Stmt::Local(local) => local.then(continuation, ctx),
            Stmt::Item(item) => item.then(continuation, ctx),
            Stmt::Expr(expr, semi) => (expr, semi).then(continuation, ctx),
            Stmt::Macro(mac) => mac.then(continuation, ctx),
        }
    }
}

// ---------------------------------------------------------------------------
// Local (`let pat [= init] [else { ... }];`)
// ---------------------------------------------------------------------------

impl CpsStep for Local {
    fn then(self, continuation: CpsContinuation, ctx: &mut CpsCtx) -> Result<CpsContinuation> {
        let Local {
            attrs,
            let_token,
            modifiers,
            pat,
            mut init,
            semi_token,
        } = self;
        assert_no_attrs(attrs)?;

        let Some(init) = init.take() else {
            // `let pat;` — no initializer, keep as-is.
            return Ok(CpsStmtLocal {
                let_token,
                modifiers,
                pat,
                init: None,
                semi_token,
            } + continuation);
        };

        match init.diverge {
            None => init.expr.with_hoisting(
                |expr, _ctx| {
                    let rebind = CpsStmtLocal {
                        let_token,
                        modifiers,
                        pat,
                        init: Some(CpsLocalInit {
                            eq_token: init.eq_token,
                            expr,
                        }),
                        semi_token,
                    };
                    Ok(rebind + continuation)
                },
                ctx,
            ),
            Some(diverge) => init.expr.with_hoisting(
                |expr, ctx| {
                    let success_arm = make_arm(pat, continuation);
                    let diverge_arm = make_arm(wildcard_pat(), diverge.converge(ctx)?);
                    let match_ = make_match(expr, vec![success_arm, diverge_arm]);
                    let match_ = CpsTerminator::Scoped(CpsTerminatorScoped {
                        scope_id: None,
                        scoped: match_,
                        scope_continuation: None,
                    });
                    Ok(match_)
                },
                ctx,
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// Item
// ---------------------------------------------------------------------------

impl CpsStep for Item {
    #[inline]
    fn then(self, continuation: CpsContinuation, _ctx: &mut CpsCtx) -> Result<CpsContinuation> {
        // help prevent redefining macros
        if let Item::Macro(i) = self {
            return Err(CpsError::UnallowedMacro(i.mac).into());
        }

        // Item definitions don't diverge.
        Ok(self + continuation)
    }
}

// ---------------------------------------------------------------------------
// StmtExpr (`expr;` or tail `expr`)
// ---------------------------------------------------------------------------

impl CpsStep for (Expr, Option<Semi>) {
    fn then(self, mut continuation: CpsContinuation, ctx: &mut CpsCtx) -> Result<CpsContinuation> {
        let (expr, semi) = self;

        // This doesn't solve
        // a (not deliminated)
        // b
        // syn might be able to reject the simple case but won't in
        // loop... inert should catch this for now
        if semi.is_some() && continuation.is_empty() {
            continuation = unit_expr() + continuation;
        }
        expr.then(continuation, ctx)
    }
}

// ---------------------------------------------------------------------------
// StmtMacro (`mac!(...);`)
// ---------------------------------------------------------------------------

impl CpsStep for StmtMacro {
    fn then(self, continuation: CpsContinuation, ctx: &mut CpsCtx) -> Result<CpsContinuation> {
        assert_no_attrs(self.attrs)?;
        if let Some(directive) = parse_borrow_directive(&self.mac)? {
            let d = match directive {
                PinBorrowDirective::LocalStart(start) => {
                    CpsStmt::from(CpsStmtDirective::from(CpsStmtDirectiveLocalPin {
                        input: start,
                    }))
                }
                PinBorrowDirective::LocalEnd(end) => {
                    CpsStmt::from(CpsStmtDirective::from(CpsStmtDirectiveLocalPinEnd {
                        input: end,
                    }))
                }
            };

            return Ok(d + continuation);
        }

        let mac = assert_valid_macro(self.mac, ctx)?;

        let cps_mac = CpsStmtMacro {
            mac,
            semi_token: self.semi_token,
        };
        Ok(CpsStmt::Plain(Box::new(cps_mac.into())) + continuation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::CpsError;
    use syn::{Stmt, parse_quote};

    #[test]
    fn unallowed_stmt_macro_rejected() {
        let stmt: Stmt = parse_quote!(unknown_macro!(););
        let err: Result<CpsContinuation> = stmt.step();
        let err = err.unwrap_err();
        assert!(matches!(*err, CpsError::UnallowedMacro(..)));
    }

    #[test]
    fn let_with_attrs_rejected() {
        let stmt: Stmt = parse_quote!(#[inline] let x = 1;);
        let err: Result<CpsContinuation> = stmt.step();
        let err = err.unwrap_err();
        assert!(matches!(*err, CpsError::UnallowedAttributes(..)));
    }

    #[test]
    fn let_else_with_panic() {
        let stmt: Stmt = parse_quote!(let Some(x) = opt else { panic!("boom"); };);

        let err: Result<CpsContinuation> = stmt.step();

        assert!(matches!(*err.unwrap_err(), CpsError::UnclearDivergence(_)));
    }

    #[test]
    fn test_let_else_divergence() {
        let block: syn::Block = parse_quote! {
            {
                let Some(_) = fetch() else { 2 };
                5
            }
        };

        let err: Result<CpsContinuation> = block.step();
        assert!(matches!(*err.unwrap_err(), CpsError::UnclearDivergence(_)));
    }
}
