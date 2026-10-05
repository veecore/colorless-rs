//! Generic recursive signal enum for loop nesting levels 1..N.

use crate::level0::Never;

/// Control-flow signal at loop nesting level `N`.
///
/// Encodes three possible control-flow actions without runtime depth counters:
/// 1. [`Self::BreakLocal`]: Exits the current loop level `N`.
/// 2. [`Self::ContinueParent`]: Jumps to the header of the enclosing parent loop `N-1`.
/// 3. [`Self::Outer`]: Bubbles up an exit targeting an outer loop or the function boundary.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
#[must_use]
pub enum LnSignal<Bn, Cp, Ln_1> {
    /// Break from the current loop with the associated value.
    BreakLocal(Bn),

    /// Continue the immediately enclosing loop.
    ContinueParent(Cp),

    /// Propagate a signal to an enclosing control-flow scope.
    Outer(Ln_1),
}

impl<Bn, Cp, Ln_1> LnSignal<Bn, Cp, Ln_1> {
    /// Creates a signal that breaks from the current loop.
    #[inline(always)]
    pub const fn break_local(b: Bn) -> Self {
        Self::BreakLocal(b)
    }

    /// Creates a signal that continues the immediately enclosing loop.
    #[inline(always)]
    pub const fn continue_parent(c: Cp) -> Self {
        Self::ContinueParent(c)
    }

    /// Wraps an outer signal for propagation to enclosing scopes.
    #[inline(always)]
    pub const fn outer(l: Ln_1) -> Self {
        Self::Outer(l)
    }
}

/// A signal type that cannot break from the current loop.
pub type LnSignalNoBreakLocal<Cp, Ln_1> = LnSignal<Never, Cp, Ln_1>;

/// A signal type that cannot continue the parent loop.
pub type LnSignalNoContinueParent<Bn, Ln_1> = LnSignal<Bn, Never, Ln_1>;

/// A signal type that can neither break locally nor continue the parent.
pub type LnSignalNoBreakLocalNoContinueParent<Ln_1> = LnSignal<Never, Never, Ln_1>;
