//! Continuation buffer holding a sequence of lowered statements and an optional terminator.

use crate::ir::CpsStmt;
use alloc::collections::VecDeque;

#[derive(Clone, Debug, Default)]
pub struct CpsContinuation {
    pub leading_stmts: VecDeque<CpsStmt>,
    pub always_diverges: bool,
}

impl CpsContinuation {
    #[inline]
    pub fn empty() -> Self {
        Self::default()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.leading_stmts.is_empty()
    }
}

extern crate alloc;
