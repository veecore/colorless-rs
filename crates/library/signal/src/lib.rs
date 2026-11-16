//! # `cps-signal`
//!
//! Typed control-flow signals used by Colorless to lower loops, breaks, continues,
//! and returns into environments without native jump propagation.
//!
//! ## Architecture
//!
//! The signal types encode the maximum number of enclosing loop scopes that a
//! control-flow event may need to cross:
//!
//! - [`L0Signal`]: Outermost loop level (local break or function return).
//! - [`LnSignal`]: Recursive loop level `N` (break local, continue parent, or bubble outer).
//! - [`L1Signal`] .. [`L9Signal`]: Concrete, ergonomic type aliases generated over [`LnSignal`].

#![allow(non_camel_case_types)]
#![no_std]

pub mod level0;
pub mod level_n;
pub mod levels;
pub mod traits;

use core::convert::Infallible;
pub use level_n::*;
pub use level0::*;
pub use levels::*;
pub use traits::*;

/// Placeholder for the uninhabited never type (`!`).
///
/// This is used for signal payloads that cannot occur at a particular
/// instantiation.
pub type Never = Infallible;

#[cfg(test)]
mod tests {
    use super::*;
    use core::ops::ControlFlow;

    #[allow(unused)]
    fn fold<C, R>(mut state: C, mut f: impl FnMut(C) -> ControlFlow<R, C>) -> R {
        loop {
            match f(state) {
                ControlFlow::Break(r) => return r,
                ControlFlow::Continue(c) => state = c,
            }
        }
    }

    #[test]
    fn local_break() {
        /*
        loop {
            break 42
        }
        */
        let signal = fold((), |_| {
            return ControlFlow::Break(break_0(42));
        });

        match signal {
            L0SignalNoOuter::Break(value) => {
                assert_eq!(value, 42);
            }
        }
    }

    #[test]
    fn parent_continue() {
        /*
        let mut iterations = 0;
        'a: loop {
            iterations += 1;
            if iterations > 3 {
                break;
            }
            loop {
                continue 'a;
            }
        }
        */
        let mut iterations = 0;
        let signal = fold((), |_| {
            iterations += 1;
            if iterations > 3 {
                return ControlFlow::Break(break_0(()));
            }
            let signal = fold((), |_| {
                return ControlFlow::Break(continue_0(()));
            });

            match signal {
                L1SignalNoOuterNoBreak::ContinueParent(c) => ControlFlow::Continue(c),
            }
        });

        match signal {
            L0SignalNoOuter::Break(()) => {}
        }

        assert_eq!(iterations, 4);
    }

    #[test]
    fn break_to_parent() {
        /*
        'a: loop {
            loop {
                break 'a 42;
            }
        };
        */
        let value = fold((), |_| {
            let signal = fold((), |_| {
                return ControlFlow::Break(L1Signal::break_0(42));
            });

            match signal {
                LnSignalNoBreakNoContinueParent::Outer(signal) => ControlFlow::Break(signal),
            }
        });

        match value {
            L0SignalNoOuter::Break(value) => {
                assert_eq!(value, 42);
            }
        }
    }

    #[test]
    fn break_to_outermost_loop() {
        /*
        'a: loop {
            'b: loop {
                'c: loop {
                    break 'a 42;
                }
            }
        }
        */
        let x = fold((), |_| {
            let signal = fold((), |_| {
                let signal = fold((), |_| {
                    return ControlFlow::Break(L2Signal::break_0(42));
                });
                match signal {
                    LnSignalNoBreakNoContinueParent::Outer(signal) => ControlFlow::Break(signal),
                }
            });

            match signal {
                LnSignalNoBreakNoContinueParent::Outer(signal) => ControlFlow::Break(signal),
            }
        });

        match x {
            L0SignalNoOuter::Break(value) => {
                assert_eq!(value, 42);
            }
        }
    }

    #[test]
    fn local_does_not_propagate() {
        /*
        loop {
            loop {
                 ..
                     ..
                         loop {
                             break 42
                         }
                     ..
                 ..
            }
        }
        */
        let signal = L9Signal::break_9(42);
        match signal {
            L9SignalNoOuterNoContinueParent::Break(value) => assert_eq!(value, 42),
        }
    }

    #[test]
    fn continue_parent_cascade() {
        /*
        let mut total = 0;
        'outer: loop {
            total += 1;
            if total >= 10 {
                break;
            }
            loop {
                loop {
                     continue 'outer;
                }
            }
        }

        // The signal should cross two loop boundaries before reaching the
        // loop that owns it.
        */

        let mut total = 0;
        let signal = fold((), |_| {
            total += 1;
            if total >= 10 {
                return ControlFlow::Break(break_0(()));
            }
            let signal = fold((), |_| {
                let signal = fold((), |_| {
                    return ControlFlow::Break(continue_0(()));
                });

                match signal {
                    LnSignalNoBreakNoContinueParent::Outer(o) => ControlFlow::Break(o),
                }
            });

            match signal {
                L1SignalNoOuterNoBreak::ContinueParent(()) => ControlFlow::Continue(()),
            }
        });

        match signal {
            L0SignalNoOuter::Break(()) => {
                assert_eq!(total, 10);
            }
        }
    }
}
