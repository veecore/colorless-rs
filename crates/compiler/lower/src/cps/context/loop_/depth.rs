use crate::cps::context::CpsLoopFacts;
use std::fmt::Display;
use std::ops::{Add, AddAssign};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CpsLoopDepth(pub usize);

impl CpsLoopDepth {
    pub(crate) fn is_outermost(self) -> bool {
        self.0 == 0
    }

    pub(crate) fn checked_sub(self, rhs: usize) -> Option<Self> {
        self.0.checked_sub(rhs).map(Self)
    }
}

impl CpsLoopFacts {
    pub(crate) fn is_outermost(&self) -> bool {
        self.loop_id.depth.is_outermost()
    }
}

impl AddAssign<usize> for CpsLoopDepth {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs
    }
}

impl Add<usize> for CpsLoopDepth {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl Display for CpsLoopDepth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
