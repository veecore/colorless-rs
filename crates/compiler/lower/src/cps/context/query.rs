use crate::cps::CpsCtx;
use crate::cps::context::{CpsControlFlowTarget, CpsLoopTarget};
use syn::Lifetime;

#[derive(Debug)]
pub struct ResolvedControlFlowTarget<'a> {
    target_idx: usize,
    current_loop_idx: Option<usize>,
    ctx: &'a mut CpsCtx,
}

pub struct CurrentLoop<'a> {
    current_loop_idx: usize,
    ctx: &'a mut CpsCtx,
}

impl CpsCtx {
    #[inline]
    #[must_use]
    pub(crate) fn resolve_control_flow(
        &mut self,
        label: Option<Lifetime>,
    ) -> Option<ResolvedControlFlowTarget<'_>> {
        let current_loop = self.current_loop_position();

        match label {
            None => {
                // current_loop_idx has to exist for implicit target
                let current_loop_idx = current_loop?;

                // The target_loop is also the current loop
                Some(ResolvedControlFlowTarget {
                    target_idx: current_loop_idx,
                    current_loop_idx: Some(current_loop_idx),
                    ctx: self,
                })
            }
            Some(label) => {
                let target_idx = self.control_flow_position(label)?;
                Some(ResolvedControlFlowTarget {
                    target_idx,
                    current_loop_idx: current_loop,
                    ctx: self,
                })
            }
        }
    }

    #[inline]
    #[must_use]
    pub(crate) fn current_loop(&mut self) -> Option<CurrentLoop<'_>> {
        Some(CurrentLoop {
            current_loop_idx: self.current_loop_position()?,
            ctx: self,
        })
    }

    #[inline]
    #[must_use]
    pub(crate) fn control_flow_position(&self, label: Lifetime) -> Option<usize> {
        self.control_flow_context
            .stack
            .iter()
            .rposition(|s| s.is_control_flow_target(&label))
    }

    #[inline]
    #[must_use]
    pub(crate) fn current_loop_position(&self) -> Option<usize> {
        self.control_flow_context
            .stack
            .iter()
            .rposition(|s| s.is_loop())
    }

    #[inline]
    #[must_use]
    fn index_control_flow(&mut self, idx: usize) -> &mut CpsControlFlowTarget {
        &mut self.control_flow_context.stack[idx]
    }

    #[inline]
    #[must_use]
    fn index_loop_unchecked(&mut self, idx: usize) -> &mut CpsLoopTarget {
        match &mut self.control_flow_context.stack[idx] {
            CpsControlFlowTarget::Loop(target_loop) => target_loop,
            _ => unreachable!(),
        }
    }
}

impl ResolvedControlFlowTarget<'_> {
    #[inline]
    #[must_use]
    pub(crate) fn target(&mut self) -> &mut CpsControlFlowTarget {
        self.ctx.index_control_flow(self.target_idx)
    }

    #[inline]
    #[must_use]
    pub(crate) fn current_loop(&mut self) -> Option<&mut CpsLoopTarget> {
        Some(self.ctx.index_loop_unchecked(self.current_loop_idx?))
    }

    #[inline]
    #[must_use]
    pub(crate) fn target_child_loop(&mut self) -> Option<&mut CpsLoopTarget> {
        Some(
            self.ctx
                .index_loop_unchecked(self.target_child_loop_position()?),
        )
    }

    #[inline]
    #[must_use]
    fn target_child_loop_position(&self) -> Option<usize> {
        self.ctx.control_flow_context.stack[self.target_idx + 1..]
            .iter()
            .position(|t| t.is_loop())
            .map(|rel_idx| self.target_idx + 1 + rel_idx)
    }
}

impl<'a> CurrentLoop<'a> {
    pub(crate) fn inner(&mut self) -> &mut CpsLoopTarget {
        self.ctx.index_loop_unchecked(self.current_loop_idx)
    }
}

impl CpsControlFlowTarget {
    #[inline]
    #[must_use]
    pub(crate) fn is_control_flow_target(&self, l: &Lifetime) -> bool {
        let label = match self {
            CpsControlFlowTarget::Block(b) => Some(&b.label),
            CpsControlFlowTarget::Loop(l) => l.label.as_ref(),
        };
        label.as_ref().map(|s| &s.name) == Some(l)
    }

    #[inline]
    #[must_use]
    pub(crate) fn is_loop(&self) -> bool {
        matches!(self, CpsControlFlowTarget::Loop(_))
    }
}

impl AsMut<CpsCtx> for ResolvedControlFlowTarget<'_> {
    fn as_mut(&mut self) -> &mut CpsCtx {
        self.ctx
    }
}

impl AsMut<CpsCtx> for CurrentLoop<'_> {
    fn as_mut(&mut self) -> &mut CpsCtx {
        self.ctx
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cps::context::loop_::depth::CpsLoopDepth;
    use syn::parse_quote;

    #[test]
    fn labeled_resolve_test() {
        let mut ctx = CpsCtx::default();
        ctx.register_loop(Some(&parse_quote!('a:)));
        ctx.register_block(&parse_quote!('b:), None);
        ctx.register_loop(Some(&parse_quote!('c:)));

        let mut resolved = ctx.resolve_control_flow(parse_quote!('a)).unwrap();

        assert_eq!(resolved.current_loop_idx, Some(2));
        assert_eq!(resolved.target_idx, 0);
        assert_eq!(resolved.current_loop().unwrap().depth(), CpsLoopDepth(1));
        assert_eq!(
            resolved.target().loop_mut().unwrap().depth(),
            CpsLoopDepth(0)
        );
    }

    #[test]
    fn unlabeled_resolve_test() {
        let mut ctx = CpsCtx::default();
        ctx.register_loop(Some(&parse_quote!('a:)));
        ctx.register_block(&parse_quote!('b:), None);
        ctx.register_loop(None);

        let mut resolved = ctx.resolve_control_flow(None).unwrap();
        assert_eq!(resolved.current_loop_idx, Some(2));
        assert_eq!(resolved.target_idx, 2);
        assert_eq!(resolved.current_loop().unwrap().depth(), CpsLoopDepth(1));
        assert_eq!(
            resolved.target().loop_mut().unwrap().depth(),
            CpsLoopDepth(1)
        );
    }

    #[test]
    fn label_shadow_test() {
        let mut ctx = CpsCtx::default();
        ctx.register_loop(Some(&parse_quote!('a:)));
        ctx.register_loop(Some(&parse_quote!('a:)));

        let resolved = ctx.resolve_control_flow(parse_quote!('a));
        assert!(resolved.is_some());
        let resolved = resolved.unwrap();

        assert_eq!(resolved.target_idx, 1);
        assert_eq!(resolved.current_loop_idx, Some(1));

        let _ = ctx.finish_loop(Some(&parse_quote!('a:)));

        let resolved = ctx.resolve_control_flow(parse_quote!('a));
        assert!(resolved.is_some());
        let resolved = resolved.unwrap();

        assert_eq!(resolved.target_idx, 0);
        assert_eq!(resolved.current_loop_idx, Some(0));
    }

    #[test]
    fn all_loop_resolve_test() {
        let mut ctx = CpsCtx::default();
        ctx.register_loop(Some(&parse_quote!('a:)));
        ctx.register_loop(Some(&parse_quote!('b:)));
        ctx.register_loop(Some(&parse_quote!('c:)));
        ctx.register_loop(Some(&parse_quote!('d:)));

        let mut resolved = ctx.resolve_control_flow(parse_quote!('b)).unwrap();

        assert_eq!(resolved.target_idx, 1);
        assert_eq!(resolved.current_loop_idx, Some(3));

        let target = resolved.target().loop_mut().unwrap();
        assert_eq!(target.label, Some(parse_quote!('b:)));
        assert_eq!(target.depth(), CpsLoopDepth(1));

        let resolved = resolved.current_loop().unwrap();
        assert_eq!(resolved.label, Some(parse_quote!('d:)));
        assert_eq!(resolved.depth(), CpsLoopDepth(3));
    }

    #[test]
    fn current_loop_test() {
        let mut ctx = CpsCtx::default();
        ctx.register_loop(Some(&parse_quote!('a:)));
        ctx.register_block(&parse_quote!('b:), None);
        ctx.register_loop(Some(&parse_quote!('c:)));

        let mut resolved = ctx.current_loop().unwrap();
        assert_eq!(resolved.inner().label, Some(parse_quote!('c:)));
        assert_eq!(resolved.inner().depth(), CpsLoopDepth(1));
    }

    #[test]
    fn resolve_labeled_block_inside_loop_fails_due_to_limit_hint() {
        let mut ctx = CpsCtx::default();

        // 1. Enter a loop (pushed at index 0)
        ctx.register_loop(Some(&parse_quote!('my_loop:)));

        // 2. Inside the loop, enter a labeled block (pushed at index 1)
        //
        // Equivalent Rust code:
        // 'my_loop: loop {
        //     'my_block: {
        //         break 'my_block;
        //     }
        // }
        ctx.register_block(&parse_quote!('my_block:), None);

        // 3. Attempt to resolve 'my_block to break out of it
        let resolved = ctx.resolve_control_flow(Some(parse_quote!('my_block)));

        // EXPECTATION: Should find 'my_block at stack index 1.
        // ACTUAL: Panics! `resolved` is None.
        assert!(
            resolved.is_some(),
            "Failed to resolve labeled block 'my_block defined inside an active loop"
        );
    }
}
