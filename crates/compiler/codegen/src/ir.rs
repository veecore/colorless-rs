use crate::Result;
use crate::context::FoldContext;
use crate::runtime_call::thunk;
use crate::util::block;
use std::collections::VecDeque;
use syn::{Expr, Stmt, parse_quote};

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

        pub fn declare_macros(ctx: &FoldContext) -> Stmt {
            let krate = &ctx.config.crate_path;
            // for inert code, we just emit a different use
            parse_quote! {
                use #krate::{
                    rb, borrow, unborrow, keep_alive, slot, take_slot, rb as rbv
                };
            }
        }

        // folded.stmts.push_front(ctx.use_color());
        if ctx.config.declare_borrow_macros {
            folded.stmts.push_front(declare_macros(ctx));
        }

        let body = match folded.output.yield_point {
            YieldPoint::Plain => {
                if let Some(block) = ctx.config.original_block_plain.take() {
                    block.stmts
                } else {
                    folded.stmts.into()
                }
            }
            YieldPoint::Colored => folded.stmts.into(),
        };

        if ctx.config.no_wrap {
            match folded.output.yield_point {
                YieldPoint::Colored => {
                    // already lowered - has no native control-flows
                    Ok(block(body))
                }
                YieldPoint::Plain => {
                    let func = thunk(!ctx.config.no_capture, body);

                    Ok(ctx.call("Ready", "ready_fn", [func]))
                }
            }
        } else {
            let body = thunk(!ctx.config.no_capture, body);

            Ok(ctx.call(
                "Wrap",
                match folded.output.yield_point {
                    YieldPoint::Colored => "wrap_flatten",
                    YieldPoint::Plain => "wrap",
                },
                [body],
            ))
        }
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
