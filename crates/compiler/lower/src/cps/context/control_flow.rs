//! Scope resolution and upward signal bubbling algorithms across loop scopes.

use crate::{
    cps::CpsCtx, cps::context::loop_::effects::CpsLoopControlEffects,
    ir::scoped::CpsScopedContinuation,
};
use std::ops::{Deref, DerefMut};
use syn::Label;

pub use crate::cps::context::loop_::depth::CpsLoopDepth;

#[derive(Debug)]
pub struct ControlFlowContext {
    /// Stack of loop / labeled block scopes, innermost last. Empty outside any loop.
    pub(super) stack: Vec<CpsControlFlowTarget>,
    pub(super) next_loop_id: CpsLoopId,
}

#[derive(Debug)]
#[must_use]
pub(crate) enum CpsControlFlowTarget {
    Block(CpsBlockTarget),
    Loop(CpsLoopTarget),
}

/// Metadata for one active labeled block on the lowering stack.
#[derive(Debug)]
#[must_use]
pub(crate) struct CpsBlockTarget {
    pub(super) label: Label,
    pub(crate) continuation: Option<CpsScopedContinuation>,
}

/// Metadata for one active loop on the lowering stack.
#[derive(Debug)]
#[must_use]
pub(crate) struct CpsLoopTarget {
    pub(super) facts: CpsLoopFacts,
    /// The original user-written label of this loop, if any.
    pub(super) label: Option<Label>,
}

// Consider renaming
// Things we generate for the loop
#[derive(Clone, Debug)]
pub struct CpsLoopFacts {
    pub(super) loop_id: CpsLoopId,
    pub(super) control_effects: CpsLoopControlEffects,
}

/// Stable identity assigned to a loop while lowering.
///
/// `depth` describes the runtime signal level, while `index` distinguishes
/// sequential loops which happen to occupy the same nesting depth.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CpsLoopId {
    pub(super) index: usize,
    pub(super) depth: CpsLoopDepth,
}

impl Default for ControlFlowContext {
    fn default() -> Self {
        Self {
            stack: Default::default(),
            next_loop_id: CpsLoopId {
                index: 0,
                depth: CpsLoopDepth(0),
            },
        }
    }
}
impl CpsCtx {
    /// Push a new labeled block scope.
    pub(crate) fn register_block(&mut self, label: &Label, target: Option<CpsScopedContinuation>) {
        self.control_flow_context
            .stack
            .push(CpsControlFlowTarget::Block(CpsBlockTarget {
                label: label.clone(),
                continuation: target,
            }));
    }

    /// Pop the innermost labeled block scope.
    pub(crate) fn finish_block(&mut self, label: &Label) {
        match self
            .control_flow_context
            .stack
            .pop()
            .expect("internal error: finish_block on empty stack")
        {
            CpsControlFlowTarget::Block(block) => {
                assert_eq!(&block.label, label);
            }
            CpsControlFlowTarget::Loop(_) => {
                panic!("internal error: a call to register_loop is not followed by finish_loop");
            }
        }
    }

    /// Enters a loop scope. `user_label` is preserved only for target lookup.
    pub(crate) fn register_loop(&mut self, user_label: Option<&Label>) -> CpsLoopId {
        let loop_id = self.control_flow_context.next_loop_id.fetch_advance();

        let l = CpsControlFlowTarget::Loop(CpsLoopTarget {
            label: user_label.cloned(),
            facts: CpsLoopFacts {
                loop_id,
                control_effects: Default::default(),
            },
        });

        self.control_flow_context.stack.push(l);

        loop_id
    }

    /// Pop the innermost loop scope.
    #[inline]
    pub(crate) fn finish_loop(&mut self, user_label: Option<&Label>) -> CpsLoopTarget {
        self.control_flow_context.next_loop_id.depth = self
            .control_flow_context
            .next_loop_id
            .depth
            .checked_sub(1)
            .expect("internal error: finish_loop without a matching register_loop");

        match self
            .control_flow_context
            .stack
            .pop()
            // drift in this module actually
            .expect("internal error: finish_loop without register_loop")
        {
            CpsControlFlowTarget::Block(_) => {
                panic!("internal error: a call to register_block is not followed by finish_block");
            }
            CpsControlFlowTarget::Loop(l) => {
                assert_eq!(l.label.as_ref(), user_label);
                l
            }
        }
    }
}

impl CpsControlFlowTarget {
    #[cfg(test)]
    pub(crate) fn loop_mut(&mut self) -> Option<&mut CpsLoopTarget> {
        match self {
            CpsControlFlowTarget::Loop(l) => Some(l),
            CpsControlFlowTarget::Block(_) => None,
        }
    }
}

impl CpsLoopId {
    pub fn depth(&self) -> CpsLoopDepth {
        self.depth
    }

    fn fetch_advance(&mut self) -> Self {
        let depth = self.depth;
        let index = self.index;
        self.depth += 1;
        self.index += 1;

        Self { depth, index }
    }
}

impl CpsLoopFacts {
    /// Stable identity of the loop which owns these lowering facts.
    #[inline]
    pub fn loop_id(&self) -> CpsLoopId {
        self.loop_id
    }
}

impl CpsLoopTarget {
    pub(crate) fn facts(self) -> CpsLoopFacts {
        self.facts
    }

    pub(crate) fn facts_ref(&self) -> &CpsLoopFacts {
        &self.facts
    }
}

impl Deref for CpsLoopTarget {
    type Target = CpsLoopFacts;

    fn deref(&self) -> &Self::Target {
        &self.facts
    }
}

impl DerefMut for CpsLoopTarget {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.facts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn labeled_blocks_do_not_change_the_active_loop() {
        let mut ctx = CpsCtx::default();
        let loop_label: Label = parse_quote!('outer:);
        let block_label: Label = parse_quote!('block:);

        ctx.register_loop(Some(&loop_label));
        ctx.register_block(&block_label, None);

        assert_eq!(ctx.current_loop_position(), Some(0));

        ctx.finish_block(&block_label);
        let loop_target = ctx.finish_loop(Some(&loop_label));
        assert!(loop_target.is_outermost());
    }
}
