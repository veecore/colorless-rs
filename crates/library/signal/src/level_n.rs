//! Generic recursive signal enum for loop nesting levels 1..N.

use crate::Never;

/// Control-flow signal at loop nesting level `N`.
///
/// Encodes three possible control-flow actions without runtime depth counters:
/// 1. [`Self::Break`]: Exits the current loop level `N`.
/// 2. [`Self::ContinueParent`]: Jumps to the header of the enclosing parent loop `N-1`.
/// 3. [`Self::Outer`]: Bubbles up an exit targeting an outer loop or the function boundary.
#[must_use]
#[derive(Debug)]
pub enum LnSignal<B, C, Ln_1> {
    /// Break from the current loop with the associated value.
    Break(B),

    /// Continues the current loop with state `C`.
    ContinueParent(C),

    /// Forwards a control-flow signal to an enclosing scope.
    Outer(Ln_1),
}

/// A signal type that cannot break from the current loop.
pub type LnSignalNoBreak<C, Ln_1> = LnSignal<Never, C, Ln_1>;

/// A signal type that cannot continue the parent loop.
pub type LnSignalNoContinueParent<B, Ln_1> = LnSignal<B, Never, Ln_1>;

/// A signal type that can neither break locally nor continue the parent.
pub type LnSignalNoBreakNoContinueParent<Ln_1> = LnSignal<Never, Never, Ln_1>;
