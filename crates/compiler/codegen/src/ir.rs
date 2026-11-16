use crate::Result;
use crate::context::FoldContext;
use crate::rules::borrow::declare_borrow_macros;
use crate::runtime_call::thunk;
use crate::util::block;
use std::collections::VecDeque;
use syn::{Expr, Stmt};

/// The result of folding a CPS body.
#[derive(Clone, Default)]
pub struct Folded {
    pub output: FoldedOutput,
    pub stmts: VecDeque<Stmt>,
}

// This makes Colored -> Colored representable but offers more flexibility
// the other solutions to strictly make colored -> colored unrepresentable
// required more work
#[derive(Clone, Default, Debug, PartialEq)]
pub struct FoldedOutput {
    pub yield_point: YieldPoint,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub enum YieldPoint {
    Colored,
    #[default]
    Plain,
}

impl Folded {
    pub fn hole() -> Folded {
        Folded::default()
    }

    pub fn is_hole(&self) -> bool {
        self.stmts.is_empty()
    }

    pub fn into_expr(self) -> Expr {
        block(self.stmts)
    }

    pub fn finish(self, ctx: &mut FoldContext) -> Result<Expr> {
        let mut folded = self;

        // folded.stmts.push_front(ctx.use_color());
        if ctx.config.declare_borrow_macros {
            folded.stmts.push_front(declare_borrow_macros(ctx));
        }

        let (method, body) = match folded.output.yield_point {
            YieldPoint::Plain => ("wrap", {
                if let Some(block) = ctx.config.original_block_plain.take() {
                    block.stmts
                } else {
                    folded.stmts.into()
                }
            }),
            YieldPoint::Colored => ("wrap_flatten", folded.stmts.into()),
        };

        let body = thunk(!ctx.config.no_capture, body);

        Ok(ctx.call("Wrap", method, [body]))
    }
}

impl FoldedOutput {
    pub fn requires_color(&self) -> bool {
        matches!(self.yield_point, YieldPoint::Colored)
    }
    pub fn colored() -> FoldedOutput {
        FoldedOutput {
            yield_point: YieldPoint::Colored,
        }
    }
}
