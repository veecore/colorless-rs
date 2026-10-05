//! Level-0 signal representing the outermost loop level and function boundary.

use crate::traits::{CanBreak0, CanReturn};
use core::convert::Infallible;

/// Placeholder for the uninhabited never type (`!`).
pub type Never = Infallible;

/// Control-flow signal at the outermost loop level.
///
/// A level-0 signal has nowhere else to propagate to except the function
/// boundary. [`L0Signal::BreakLocal`] represents a break from the outermost
/// loop, while [`L0Signal::Return`] represents a function return.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
#[must_use]
pub enum L0Signal<B = Never, R = Never> {
    /// Break from the current (outermost) loop with `B`.
    BreakLocal(B),

    /// Return from the enclosing function with `R`.
    Return(R),
}

impl<B, R> L0Signal<B, R> {
    /// Creates a signal that breaks from the outermost loop.
    #[inline(always)]
    pub const fn break_0(value: B) -> Self {
        Self::BreakLocal(value)
    }

    /// Creates a signal that returns from the enclosing function.
    #[inline(always)]
    pub const fn return_(value: R) -> Self {
        Self::Return(value)
    }

    /// Alias for [`L0Signal::break_0`].
    #[inline(always)]
    pub const fn break_local(value: B) -> Self {
        Self::BreakLocal(value)
    }
}

/// A level-0 signal that cannot return from the function.
pub type L0SignalNoReturn<B> = L0Signal<B>;

/// A level-0 signal that cannot break from its local loop.
pub type L0SignalNoBreakLocal<R> = L0Signal<Never, R>;

/// A level-0 signal that can neither return nor break locally.
pub type L0SignalNoReturnNoBreakLocal = L0Signal;

impl<B0, R> CanBreak0 for L0Signal<B0, R> {
    type B0 = B0;

    #[inline(always)]
    fn break_0(b: Self::B0) -> Self {
        Self::break_0(b)
    }
}

impl<B0, R> CanReturn for L0Signal<B0, R> {
    type R = R;

    #[inline(always)]
    fn return_(r: Self::R) -> Self {
        Self::return_(r)
    }
}
