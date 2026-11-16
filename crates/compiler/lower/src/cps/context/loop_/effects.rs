use crate::cps::context::{CpsLoopFacts, CpsLoopId};
use std::collections::HashSet;

/// Control-flow facts accumulated while a continuation is being lowered.
#[derive(Clone, Debug, Default)]
pub struct CpsLoopControlEffects {
    breaks_self: bool,
    continues_parent: bool,
    propagates_return: bool,
    // targeted_loops answers false positives when it only continues_parent
    targeted_loops: HashSet<CpsLoopId>,
}

impl CpsLoopFacts {
    pub(crate) fn propagate_reach_on(&self, target: &mut Self) {
        target.absorb_reach_unchecked(&self.control_effects)
    }

    fn absorb_reach_unchecked(&mut self, influencer: &CpsLoopControlEffects) {
        self.control_effects.propagates_return |= influencer.propagates_return;
        // self is now parent here
        // and influencer child... parent is trying to inherit things out of its reach
        // that its child reaches so we need to remove the assert in log since this
        // is valid
        if let Some(d) = influencer.outermost_reached_loop() {
            self.log_targets_loop(d)
        }
    }
}

// Tracking
impl CpsLoopFacts {
    pub(crate) fn log_breaks_self(&mut self) {
        self.control_effects.breaks_self = true;
    }

    pub(crate) fn log_continues_parent(&mut self) {
        self.control_effects.continues_parent = true;
    }

    pub(crate) fn log_returns(&mut self) {
        self.control_effects.propagates_return = true
    }

    pub(crate) fn log_targets_loop(&mut self, target: CpsLoopId) {
        debug_assert!(target.depth() <= self.loop_id.depth());
        if target.depth() != self.depth() {
            self.control_effects.targeted_loops.insert(target);
        }
    }
}

// Query
impl CpsLoopFacts {
    pub(crate) fn can_break_self(&self) -> bool {
        self.control_effects.breaks_self
    }

    pub(crate) fn can_continue_parent(&self) -> bool {
        self.control_effects.continues_parent
    }

    // colorless can call this to know if to remove the binding
    pub fn can_reach_outer_scope(&self) -> bool {
        self.control_effects.propagates_return
            || self
                .control_effects
                .targeted_loops
                .iter()
                // If we tell it to continue parent, that doesn't mean it does reach outer
                .any(|loop_| loop_.depth() != self.loop_id.depth)
    }

    pub fn reached_loops(&self) -> impl Iterator<Item = CpsLoopId> {
        self.control_effects
            .targeted_loops
            .iter()
            .filter_map(|loop_| {
                if loop_.depth() != self.depth() {
                    Some(*loop_)
                } else {
                    None
                }
            })
    }
}

impl CpsLoopControlEffects {
    pub fn outermost_reached_loop(&self) -> Option<CpsLoopId> {
        self.targeted_loops.iter().min().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cps::context::{CpsLoopDepth, CpsLoopId};

    /// Build a `CpsLoopFacts` at a given (depth, index) with no effects recorded.
    fn facts_at(depth: usize) -> CpsLoopFacts {
        CpsLoopFacts {
            loop_id: CpsLoopId {
                index: depth,
                depth: CpsLoopDepth(depth),
            },
            control_effects: CpsLoopControlEffects::default(),
        }
    }

    // ── Single-bit logging ────────────────────────────────────────────────────

    /// A fresh `CpsLoopFacts` has no effects set.
    #[test]
    fn fresh_facts_have_no_effects() {
        let f = facts_at(0);
        assert!(!f.can_break_self());
        assert!(!f.can_continue_parent());
        assert!(!f.can_reach_outer_scope());
    }

    /// `log_breaks_self` sets only the break-self bit.
    #[test]
    fn log_breaks_self_sets_break_bit() {
        let mut f = facts_at(1);
        f.log_breaks_self();
        assert!(f.can_break_self());
        assert!(!f.can_continue_parent());
        // A simple break-self does not reach outer scope
        // (no return, no cross-loop target).
        assert!(!f.can_reach_outer_scope());
    }

    /// `log_continues_parent` sets only the continue-parent bit.
    #[test]
    fn log_continues_parent_sets_continue_bit() {
        let mut f = facts_at(1);
        f.log_continues_parent();
        assert!(!f.can_break_self());
        assert!(f.can_continue_parent());
        // continue-parent alone does not constitute reaching outer scope
        // (it is handled by the signal at the parent level, not here).
        assert!(!f.can_reach_outer_scope());
    }

    /// `log_returns` causes `can_reach_outer_scope` to be true.
    #[test]
    fn log_returns_reaches_outer_scope() {
        let mut f = facts_at(1);
        f.log_returns();
        assert!(f.can_reach_outer_scope());
    }

    /// Logging the same effect twice is idempotent.
    #[test]
    fn logging_effects_is_idempotent() {
        let mut f = facts_at(1);
        f.log_breaks_self();
        f.log_breaks_self();
        assert!(f.can_break_self());
        f.log_returns();
        f.log_returns();
        assert!(f.can_reach_outer_scope());
    }

    // ── log_targets_loop / reached_loops ─────────────────────────────────────

    /// Targeting a strictly outer loop (lower depth) is recorded and surfaces
    /// through `can_reach_outer_scope` and `reached_loops`.
    #[test]
    fn targeting_outer_loop_reaches_outer_scope() {
        // Inner loop at depth 2; outer loop at depth 0.
        let outer_id = CpsLoopId {
            index: 0,
            depth: CpsLoopDepth(0),
        };
        let mut inner = facts_at(2);
        inner.log_targets_loop(outer_id);

        assert!(inner.can_reach_outer_scope());
        let reached: Vec<_> = inner.reached_loops().collect();
        assert!(reached.contains(&outer_id));
    }

    /// Targeting a loop at the same depth as self is *not* recorded in
    /// `reached_loops` (it is a same-level target, not an outer-scope escape).
    #[test]
    fn targeting_same_depth_loop_is_not_in_reached_loops() {
        let same_depth_id = CpsLoopId {
            index: 5,
            depth: CpsLoopDepth(2),
        };
        let mut f = facts_at(2);
        f.log_targets_loop(same_depth_id);

        let reached: Vec<_> = f.reached_loops().collect();
        assert!(
            reached.is_empty(),
            "same-depth target must not appear in reached_loops"
        );
    }

    // ── propagate_reach_on ───────────────────────────────────────────────────

    /// Absorbing an empty child does not erase an already-set return-reach on
    /// the parent.
    #[test]
    fn absorbing_empty_child_does_not_erase_parent_return_reach() {
        let child = facts_at(1); // no effects
        let mut parent = facts_at(0);
        parent.log_returns();

        child.propagate_reach_on(&mut parent);

        assert!(parent.can_reach_outer_scope());
    }

    /// A child's `log_returns` propagates to the parent.
    #[test]
    fn child_return_reach_propagates_to_parent() {
        let mut child = facts_at(1);
        child.log_returns();
        let mut parent = facts_at(0);

        child.propagate_reach_on(&mut parent);

        assert!(parent.can_reach_outer_scope());
    }

    /// Propagating the same child twice is idempotent (boolean OR semantics).
    #[test]
    fn propagating_the_same_child_twice_is_idempotent() {
        let mut child = facts_at(1);
        child.log_returns();
        let mut parent = facts_at(0);

        child.propagate_reach_on(&mut parent);
        let after_once = parent.can_reach_outer_scope();
        child.propagate_reach_on(&mut parent);

        assert_eq!(parent.can_reach_outer_scope(), after_once);
    }

    /// A child targeting an outer loop propagates that target outward so the
    /// grandparent can observe it through `can_reach_outer_scope`.
    #[test]
    fn child_cross_loop_target_propagates_to_parent() {
        // Three levels: grandparent (depth 0), parent (depth 1), child (depth 2).
        let grandparent_id = CpsLoopId {
            index: 0,
            depth: CpsLoopDepth(0),
        };
        let mut child = facts_at(2);
        child.log_targets_loop(grandparent_id);

        let mut parent = facts_at(1);
        child.propagate_reach_on(&mut parent);

        // Parent should now also know that something below it escapes to depth 0.
        assert!(parent.can_reach_outer_scope());
    }

    // ── outermost_reached_loop ───────────────────────────────────────────────

    /// When multiple outer loops are targeted, `outermost_reached_loop` returns
    /// the one with the lowest (shallowest) depth.
    #[test]
    fn outermost_reached_loop_returns_min_depth() {
        let near_id = CpsLoopId {
            index: 1,
            depth: CpsLoopDepth(1),
        };
        let far_id = CpsLoopId {
            index: 0,
            depth: CpsLoopDepth(0),
        };
        let mut f = facts_at(3);
        f.log_targets_loop(near_id);
        f.log_targets_loop(far_id);

        assert_eq!(f.control_effects.outermost_reached_loop(), Some(far_id));
    }

    /// A loop with no targets reports `None` from `outermost_reached_loop`.
    #[test]
    fn outermost_reached_loop_is_none_when_no_targets() {
        let f = facts_at(2);
        assert_eq!(f.control_effects.outermost_reached_loop(), None);
    }
}
