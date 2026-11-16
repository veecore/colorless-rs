use crate::Fold;
use crate::context::FoldContext;
use crate::ir::Folded;
use crate::util::stmts;
use colorless_lower::ir::CpsBlock;
use colorless_lower::ir::scoped::{CpsScopedBlock, CpsScopedUnsafe};
use syn::{Block, Expr, ExprUnsafe};

impl Fold for CpsBlock {
    type Output = Folded;

    fn fold(self, _below: (), ctx: &mut FoldContext) -> crate::Result<Self::Output> {
        let brace_token = self.brace_token;
        let folded = self.body.fold((), ctx)?;

        // A source block remains a physical Rust scope even when no deferred
        // continuation happens to target it. In particular, its locals must be
        // destroyed before a following continuation is entered. The structural
        // parent may subsequently resume a targeted thunk around this block,
        // but it must never have to reconstruct the braces themselves.
        let block = Expr::Block(syn::ExprBlock {
            attrs: Vec::new(),
            label: None,
            block: Block {
                brace_token,
                stmts: folded.stmts.into(),
            },
        });

        Ok(Folded {
            output: folded.output,
            stmts: stmts(block),
        })
    }
}

impl Fold for CpsScopedBlock {
    type Output = Folded;

    fn fold(self, (): (), ctx: &mut FoldContext) -> crate::Result<Self::Output> {
        let brace_token = self.block.brace_token;
        let folded = self.block.body.fold((), ctx)?;
        let block = Expr::Block(syn::ExprBlock {
            attrs: Vec::new(),
            label: None,
            block: Block {
                brace_token,
                stmts: folded.stmts.into(),
            },
        });

        Ok(Folded {
            output: folded.output,
            stmts: stmts(block),
        })
    }
}

impl Fold for CpsScopedUnsafe {
    type Output = Folded;

    fn fold(self, (): (), ctx: &mut FoldContext) -> crate::Result<Self::Output> {
        let brace_token = self.block.brace_token;
        let folded = self.block.body.fold((), ctx)?;
        let unsafe_expr = Expr::Unsafe(ExprUnsafe {
            attrs: Vec::new(),
            unsafe_token: self.unsafe_token,
            block: Block {
                brace_token,
                stmts: folded.stmts.into(),
            },
        });

        Ok(Folded {
            output: folded.output,
            stmts: stmts(unsafe_expr),
        })
    }
}
