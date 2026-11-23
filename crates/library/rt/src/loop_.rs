//! Loop step inference and diagnostic types emitted during loop lowering.

use crate::{ErasedBorrow, LocalCell, Reborrow};
use core::mem::MaybeUninit;

/// # ⚠️ Colorless Diagnostic: Undeclared Variable Captured in Loop
///
/// A variable from an outer scope was used or modified inside this loop, but was
/// **not declared in `#[loop_state(...)]`**.
///
/// ### Why did this happen?
/// In Colorless, loops are transformed into isolated, stack-safe step functions. All
/// mutable and persistent loop state must be explicitly threaded through the loop.
///
/// If an outer variable is accessed directly without being declared in `loop_state`:
/// - For **`Copy` types** (like `i32`, `bool`, `f64`), changes are made to an implicit local
///   copy and are **silently lost** on the next iteration and in subsequent code.
/// - For non-copy types, values would be moved or improperly borrowed across iterations.
///
/// To prevent this silent bug, Colorless enforces that loop closures capture **nothing**
/// from their environment by coercing them to a function pointer.
///
/// ### How to fix it:
/// Add the captured variable to your loop's state declaration so Colorless can thread it:
///
/// ```rust,ignore
/// // Change this:
/// loop {
///     counter += 1;
/// }
///
/// // To this:
/// #[loop_state(mut counter)]
/// loop {
///     counter += 1;
/// }
/// ```
///
/// If you also need to use `counter` after the loop exits, remember to add `pub`:
/// ```rust,ignore
/// #[loop_state(pub mut counter)]
/// loop {
///     counter += 1;
/// }
/// ```
#[inline(always)]
pub fn assert_non_capturing<State, Output>(_: &State, _: fn(State) -> Output) {}

/// Loop step inference helper used to guide closure inference in lowered loops.
#[inline(always)]
#[doc(hidden)]
pub const fn infer_loop_step<State, Output, Step>(_: &State, step: Step) -> Step
where
    Step: FnMut(State) -> Output,
{
    step
}

/// # ⚠️ Colorless Diagnostic: Unexported Loop State
///
/// This variable was used as state inside a loop, but was **not exported** to the
/// code following the loop.
///
/// ### Why did this happen?
/// In Colorless, continuation macros capture variables from outer scopes. If a variable
/// is modified inside a loop (especially `Copy` types like integers or booleans), continuing
/// to use it after the loop without explicitly threading it out would cause your code
/// to **silently read the stale, pre-loop value**.
///
/// To protect against silent data corruption, Colorless shadows unexported loop variables
/// with this uninitialized `Shadowed` type once the loop exits.
///
/// ### How to fix it:
/// If you intended to use the updated value of this variable after the loop terminates,
/// mark it with the **`pub`** modifier in your loop directive:
///
/// ```rust,ignore
/// // Change this:
/// #[loop_state(mut counter)]
/// loop { ... }
///
/// // To this:
/// #[loop_state(pub mut counter)]
/// loop { ... }
/// ```
///
/// If the variable is immutable after the loop, you can use `pub counter`:
/// ```rust,ignore
/// #[loop_state(pub counter)]
/// loop { ... }
/// ```
pub struct Shadowed;

#[repr(transparent)]
pub struct Slot<S> {
    // Using methods broke ?Sized
    pub maybe_uninit: MaybeUninit<S>,
}

// Marker for local wrappers with no drop
trait SlotCarrier {}

impl<S: SlotCarrier> Slot<S> {
    #[inline(always)]
    pub fn new(t: S) -> Self {
        Self {
            maybe_uninit: MaybeUninit::new(t),
        }
    }

    #[inline(always)]
    pub fn take(self) -> S {
        unsafe { self.maybe_uninit.assume_init() }
    }
}

impl<S> Default for Slot<S> {
    fn default() -> Self {
        Self {
            maybe_uninit: MaybeUninit::uninit(),
        }
    }
}

impl<'a, S: Reborrow<'a>> Reborrow<'a> for Slot<S> {
    type Target = S::Target;

    #[inline(always)]
    unsafe fn reborrow(&self) -> &'a mut Self::Target {
        unsafe { self.maybe_uninit.assume_init_ref().reborrow() }
    }
}
impl<T: ?Sized> SlotCarrier for ErasedBorrow<T> {}
impl<T: ?Sized> SlotCarrier for LocalCell<'_, T> {}

#[inline(always)]
pub fn assert_slot_carrier<S: SlotCarrier>(_t: &Slot<S>) {}

#[macro_export]
macro_rules! slot {
    ($($s:ident),*) => {
        $(
            #[must_use]
            let mut $s = $crate::__compiler::Slot {
                maybe_uninit: ::core::mem::MaybeUninit::new($s),
            };

            $crate::__compiler::assert_slot_carrier(&$s);

            let mut $s = $s;
        )*
    };
}

#[macro_export]
macro_rules! take_slot {
    ($($s:ident),*) => {
        $(
            let mut $s = match $s {
                $crate::__compiler::Slot { maybe_uninit } => unsafe {
                    maybe_uninit.assume_init()
                }
            };
        )*
    };
}

#[macro_export]
macro_rules! __slot {
    ($($s:ident),*) => {
        $(
            #[allow(unused_mut)]
            let mut $s = $s;
        )*
    };
}

#[macro_export]
macro_rules! __take_slot {
    ($($s:ident),*) => {};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_loop_step() {
        let state = 42;
        let step = infer_loop_step(&state, |s: i32| s + 1);
        let mut step = step;
        assert_eq!(step(10), 11);
    }

    #[test]
    fn test_assert_non_capturing() {
        fn non_capturing(s: i32) -> i32 {
            s * 2
        }
        assert_non_capturing(&0, non_capturing);
    }
}
