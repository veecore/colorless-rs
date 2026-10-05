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
//!
//! ## Polymorphic Capability Traits
//!
//! When transforming code using Continuation-Passing Style (CPS), continuations
//! may be duplicated across different loop nesting depths. Instead of hard-pinning
//! to a concrete signal constructor, continuations invoke polymorphic free helpers:
//!
//! ## Examples
//!
//! ### Concrete Signal Matching
//!
//! ```
//! use cps_signal::{L1Signal, LnSignal, L0SignalNoReturn};
//!
//! let signal: L1Signal<(), (), i32> = L1Signal::break_0(42);
//!
//! match signal {
//!     LnSignal::Outer(inner) => {
//!         assert_eq!(inner, L0SignalNoReturn::break_0(42));
//!     }
//!     _ => unreachable!(),
//! }
//! ```
//!
//! ### Polymorphic Signal Construction
//!
//! ```
//! use cps_signal::{break_0, return_, CanBreak0, CanReturn, L0Signal, L1Signal, L2Signal, L0SignalNoBreakLocal};
//!
//! fn emit_break<C: CanBreak0<B0 = i32>>(value: i32) -> C {
//!     break_0(value)
//! }
//!
//! let at_l0: L0Signal<i32> = emit_break(10);
//! let at_l1: L1Signal<(), (), i32> = emit_break(20);
//! let at_l2: L2Signal<(), (), (), (), i32> = emit_break(30);
//!
//! fn emit_return<C: CanReturn<R = &'static str>>(msg: &'static str) -> C {
//!     return_(msg)
//! }
//!
//! let ret_l0: L0SignalNoBreakLocal<&'static str> = emit_return("done");
//! let ret_l1: L1Signal<(), (), core::convert::Infallible, &'static str> = emit_return("done");
//! ```

#![allow(non_camel_case_types)]
#![no_std]

pub mod level0;
pub mod level_n;
pub mod levels;
pub mod traits;

pub use level0::*;
pub use level_n::*;
pub use levels::*;
pub use traits::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_break() {
        let signal: L0SignalNoReturn<i32> = break_0(42);
        match signal {
            L0SignalNoReturn::BreakLocal(value) => assert_eq!(value, 42),
            L0SignalNoReturn::Return(_) => unreachable!(),
        }
    }

    #[test]
    fn parent_continue() {
        let mut iterations = 0;
        loop {
            iterations += 1;
            if iterations > 3 {
                break;
            }
            let signal: L1SignalNoOuterNoBreakLocal<()>;
            loop {
                signal = continue_0(());
                break;
            }
            match signal {
                L1SignalNoOuterNoBreakLocal::ContinueParent(_) => continue,
                _ => unreachable!(),
            }
        }
        assert_eq!(iterations, 4);
    }

    #[test]
    fn break_to_parent() {
        let signal_a;
        loop {
            let signal_b: LnSignalNoBreakLocalNoContinueParent<L0SignalNoReturn<i32>>;
            loop {
                signal_b = break_0(42);
                break;
            }

            match signal_b {
                LnSignalNoBreakLocalNoContinueParent::Outer(parent) => {
                    signal_a = parent;
                    break;
                }
                _ => unreachable!(),
            }
        }

        match signal_a {
            L0SignalNoReturn::BreakLocal(value) => assert_eq!(value, 42),
            _ => unreachable!(),
        }
    }

    #[test]
    fn break_to_outermost_loop() {
        let signal_a;
        loop {
            let signal_b;
            loop {
                let signal_c: LnSignalNoBreakLocalNoContinueParent<LnSignalNoBreakLocalNoContinueParent<L0SignalNoReturn<i32>>>;
                loop {
                    signal_c = break_0(42);
                    break;
                }

                match signal_c {
                    LnSignalNoBreakLocalNoContinueParent::Outer(parent) => {
                        signal_b = parent;
                        break;
                    }
                    _ => unreachable!(),
                }
            }

            match signal_b {
                LnSignalNoBreakLocalNoContinueParent::Outer(parent) => {
                    signal_a = parent;
                    break;
                }
                _ => unreachable!(),
            }
        }

        match signal_a {
            L0SignalNoReturn::BreakLocal(x) => assert_eq!(x, 42),
            _ => unreachable!(),
        }
    }

    #[test]
    fn return_to_function_boundary() {
        let signal_a;
        loop {
            let signal_b;
            loop {
                let signal_c: LnSignalNoBreakLocalNoContinueParent<LnSignalNoBreakLocalNoContinueParent<L0SignalNoBreakLocal<i32>>>;
                loop {
                    signal_c = return_(42);
                    break;
                }

                match signal_c {
                    LnSignalNoBreakLocalNoContinueParent::Outer(parent) => {
                        signal_b = parent;
                        break;
                    }
                    _ => unreachable!(),
                }
            }

            match signal_b {
                LnSignalNoBreakLocalNoContinueParent::Outer(parent) => {
                    signal_a = parent;
                    break;
                }
                _ => unreachable!(),
            }
        }

        match signal_a {
            L0SignalNoBreakLocal::Return(x) => assert_eq!(x, 42),
            _ => unreachable!(),
        }
    }

    #[test]
    fn local_break_does_not_propagate() {
        let signal = L9Signal::break_9(42);
        match signal {
            L9SignalNoOuterNoContinueParent::BreakLocal(value) => assert_eq!(value, 42),
            _ => unreachable!(),
        }
    }

    #[test]
    fn continue_parent_cascade() {
        let mut total = 0;
        loop {
            total += 1;
            if total >= 10 {
                break;
            }
            let signal_b;
            loop {
                let signal_c: L2Signal<Never, Never, Never, ()> = L2Signal::continue_0(());
                match signal_c {
                    LnSignal::Outer(value) => {
                        signal_b = value;
                        break;
                    }
                    _ => unreachable!(),
                }
            }

            match signal_b {
                L1SignalNoOuterNoBreakLocal::ContinueParent(_) => continue,
                _ => unreachable!(),
            }
        }
        assert_eq!(total, 10);
    }

    #[test]
    fn polymorphic_break_helpers() {
        fn produce_break_0<C: CanBreak0<B0 = i32>>(val: i32) -> C {
            break_0(val)
        }

        fn produce_break_1<C: CanBreak1<B1 = &'static str>>(val: &'static str) -> C {
            break_1(val)
        }

        let l0: L0Signal<i32, ()> = produce_break_0(10);
        assert_eq!(l0, L0Signal::BreakLocal(10));

        let l1: L1Signal<&'static str, Never, i32, ()> = produce_break_0(20);
        assert_eq!(l1, L1Signal::Outer(L0Signal::BreakLocal(20)));

        let l1_local: L1Signal<&'static str, i32, ()> = produce_break_1("hello");
        assert_eq!(l1_local, L1Signal::BreakLocal("hello"));

        let l2_outer: L2Signal<(), Never, &'static str, i32, ()> = produce_break_1("world");
        assert_eq!(l2_outer, L2Signal::Outer(L1Signal::BreakLocal("world")));
    }

    #[test]
    fn polymorphic_continue_helpers() {
        fn produce_continue_0<C: CanContinue0>(c: C::C0) -> C {
            continue_0(c)
        }

        let l1: L1Signal<(), (), (), ()> = produce_continue_0(());
        assert_eq!(l1, L1Signal::ContinueParent(()));

        let l2: L2Signal<(), (), (), (), ()> = produce_continue_0(());
        assert_eq!(l2, L2Signal::Outer(L1Signal::ContinueParent(())));
    }

    #[test]
    fn polymorphic_return_helper() {
        fn produce_return<C: CanReturn<R = i32>>(r: i32) -> C {
            return_(r)
        }

        let l0: L0Signal<(), i32> = produce_return(99);
        assert_eq!(l0, L0Signal::Return(99));

        let l1: L1Signal<(), Never, (), i32> = produce_return(100);
        assert_eq!(l1, L1Signal::Outer(L0Signal::Return(100)));

        let l3: L3Signal<(), Never, (), Never, (), Never, (), i32> = produce_return(101);
        assert_eq!(
            l3,
            L3Signal::Outer(L2Signal::Outer(L1Signal::Outer(L0Signal::Return(101))))
        );
    }
}
