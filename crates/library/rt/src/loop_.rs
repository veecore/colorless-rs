//! Loop step inference and diagnostic types emitted during loop lowering.

/// Loop step inference helper used to guide closure inference in lowered loops.
#[inline(always)]
#[doc(hidden)]
pub const fn infer_loop_step<State, Output, Step>(_: &State, step: Step) -> Step
where
    Step: FnMut(State) -> Output,
{
    step
}

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
