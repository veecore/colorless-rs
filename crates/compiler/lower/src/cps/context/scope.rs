//! Scope stack target metadata (`LoopDepth`, `LoopTarget`, `BlockTarget`, `ControlFlowTarget`).

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct LoopDepth(pub usize);

impl LoopDepth {
    #[inline]
    pub fn is_outermost(self) -> bool {
        self.0 == 0
    }
}
