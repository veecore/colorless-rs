//! CPS lowering for plain, labeled, and unsafe blocks.

mod expr_block;
mod unsafe_;

use crate::{
    Result,
    cps::{CpsBind, CpsContinuation, CpsCtx, CpsStep},
    ir::{CpsBlock, CpsBody},
    util::cps::{make_bind, unit_expr},
};
use proc_macro2::Ident;
use syn::{Block, Stmt};

// This expects that the continuation is already grouped
impl CpsStep for Block {
    /// Lower the block *without* binding its value — the tail expression is
    /// evaluated for side-effects / divergence and then discarded.
    fn then(self, continuation: CpsContinuation, ctx: &mut CpsCtx) -> Result<CpsContinuation> {
        let stmts = self.stmts;
        let seed = continuation;

        stmts.then(seed, ctx)
    }
}

// This expects that the continuation is already grouped
impl CpsBind for Block {
    /// Lower the block, binding its tail value to `bind` before running the
    /// continuation.  When the block has no tail expression its value is `()`.
    fn and_then(
        self,
        bind: Ident,
        continuation: CpsContinuation,
        ctx: &mut CpsCtx,
    ) -> Result<CpsContinuation> {
        // The delegating `CpsBind<CpsBlock>` implementation has already ended
        // this block's scope. This implementation only lowers its statements
        // and tail into that prepared continuation.
        let mut stmts = self.stmts;

        let seed = match stmts.last() {
            Some(Stmt::Expr(_, None)) => {
                let Stmt::Expr(tail, None) = stmts.pop().unwrap() else {
                    unreachable!()
                };
                tail.and_then(bind, continuation, ctx)?
            }
            _ => {
                // Block produces `()` — synthesize `let bind = (); continuation`.
                let s = make_bind(bind, unit_expr());
                s + continuation
            }
        };

        stmts.then(seed, ctx)
    }
}

impl CpsStep<CpsBlock> for Block {
    fn then(self, c: CpsContinuation, ctx: &mut CpsCtx) -> Result<CpsBlock> {
        let brace_token = self.brace_token;
        let buf: CpsContinuation = self.then(c, ctx)?;
        Ok(CpsBlock {
            brace_token,
            body: CpsBody::from(buf),
        })
    }
}

impl CpsBind<CpsBlock> for Block {
    fn and_then(self, bind: Ident, c: CpsContinuation, ctx: &mut CpsCtx) -> Result<CpsBlock> {
        let brace_token = self.brace_token;
        let buf: CpsContinuation = self.and_then(bind, c, ctx)?;
        Ok(CpsBlock {
            brace_token,
            body: CpsBody::from(buf),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cps::CpsContinuation;
    use crate::error::CpsError;
    use crate::ir::CpsTerminator;
    use crate::ir::scoped::{CpsScoped, CpsTerminatorScoped, CpsTerminatorScopedContUse};
    use syn::{ExprBlock, ExprUnsafe, parse_quote};
    // ------------------------------------------------------------------
    // Non-diverging pass-through (ANF form is expected)
    // ------------------------------------------------------------------

    fn assert_exactly_one_block_end(block: &CpsBlock) {
        let Some(CpsTerminator::ContUse(CpsTerminatorScopedContUse { scope_continuation })) =
            block.body.terminator.as_deref()
        else {
            panic!("a non-empty source block must end in its block target")
        };

        let body = &scope_continuation.body;
        assert!(
            !matches!(body.terminator.as_deref(), Some(CpsTerminator::ContUse(_))),
            "the syntax wrapper must not add a second target around its inner Block"
        );
    }

    #[test]
    fn expression_block_wrapper_does_not_duplicate_its_block_target() {
        let expr: ExprBlock = parse_quote!({ tail() });
        let lowered: CpsContinuation = expr
            .then(CpsContinuation::unit(), &mut CpsCtx::default())
            .unwrap();

        let Some(CpsTerminator::Scoped(CpsTerminatorScoped {
            scoped: CpsScoped::Block(block),
            ..
        })) = lowered.terminator.as_deref()
        else {
            panic!("expression blocks must remain structural")
        };
        assert_exactly_one_block_end(&block.block);
    }

    #[test]
    fn unsafe_wrapper_does_not_duplicate_its_block_target() {
        let expr: ExprUnsafe = parse_quote!(unsafe { tail() });
        let lowered: CpsContinuation = expr
            .then(CpsContinuation::unit(), &mut CpsCtx::default())
            .unwrap();

        let Some(CpsTerminator::Scoped(CpsTerminatorScoped {
            scoped: CpsScoped::Unsafe(unsafe_),
            ..
        })) = lowered.terminator.as_deref()
        else {
            panic!("unsafe expressions must remain structural")
        };
        assert_exactly_one_block_end(&unsafe_.block);
    }

    #[test]
    fn labeled_block_unresolved_label_errors() {
        let block: Block = parse_quote! {
            {
                'label: {
                    break 'unknown;
                }
            }
        };
        let res: Result<CpsContinuation> = block.step();
        let err = res.unwrap_err();
        assert!(matches!(*err, CpsError::InvalidBreak(..)));
    }

    #[test]
    fn break_to_block_across_loop_is_unsupported() {
        let block: Block = parse_quote! {
            {
                'exit: {
                    loop {
                        break 'exit 42;
                    }
                }
            }
        };

        let result: Result<CpsContinuation> = block.step();
        let err = result.unwrap_err();
        assert!(matches!(*err, CpsError::InvalidBreak(_)));
    }

    #[test]
    fn block_with_attrs_rejected() {
        let expr: ExprBlock = parse_quote!(
            #[inline]
            {
                1
            }
        );
        let err = expr.step().unwrap_err();
        assert!(matches!(*err, CpsError::UnallowedAttributes(..)));
    }
}
