//! Level-0 signal representing the outermost loop level and function boundary.

use crate::{CanBreak0, CanReturn, Never};

/// Control-flow signal at the outermost loop level.
///
/// A level-0 signal has nowhere else to propagate to except the function
/// boundary. [`L0Signal::Break`] represents a break from the outermost
/// loop, while [`L0Signal::Outer`] represents a function return.
#[derive(Debug)]
#[must_use]
pub enum L0Signal<B = Never, R = Never> {
    /// Break from the current (outermost) loop with `B`.
    Break(B),

    /// Return from the enclosing function with `R`.
    Outer(R),
}

impl<B, R> L0Signal<B, R> {
    /// Creates a signal that breaks from the outermost loop.
    #[inline(always)]
    pub const fn break_0(value: B) -> Self {
        Self::Break(value)
    }

    /// Creates a signal that returns `value` from the enclosing function.
    #[inline(always)]
    pub const fn return_(value: R) -> Self {
        Self::Outer(value)
    }
}

/// A level-0 signal that cannot break from its local loop.
pub type L0SignalNoBreak<R> = L0Signal<Never, R>;

/// A level-0 signal that cannot return from the function.
pub type L0SignalNoOuter<C> = L0Signal<C, Never>;

/// A level-0 signal that can neither break locally nor return.
pub type L0SignalNoOuterNoBreak = L0Signal<Never, Never>;

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
