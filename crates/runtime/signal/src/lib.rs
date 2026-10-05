//! Typed control-flow signals used by `colorless-cps` to lower loops and other
//! control-flow constructs into environments without native jump propagation.
//!
//! # Architecture
//!
//! The signal types encode the maximum number of enclosing loop scopes that a
//! control-flow event may need to cross. For example, [`L2Signal`] can
//! represent:
//!
//! - a 'break' from the current loop ([`L2Signal::BreakLocal`]),
//! - a 'continue' of its parent loop ([`L2Signal::ContinueParent`]),
//! - a break or continue of outer loop levels,
//! - or a function return that must bubble all the way out.
//!
//! Each outer level is represented recursively through [`L2Signal::Outer`]
//! (and its equivalents on the other signal types). This keeps control
//! target information strictly in the type system rather than storing runtime
//! loop depths or dynamic jumps.
//!
//! # Unpinned Continuations & Capability Traits
//!
//! When transforming code using Continuation Passing Style (CPS), continuations
//! may be duplicated across different loop nesting depths (e.g. during AST
//! duplication for break continuation paths). If a continuation directly emitted
//! a concrete signal constructor like `L1Signal::break_0(val)`, it would be
//! hard-pinned to nesting depth 1 and could not be reused at nesting depth 2 or 0.
//!
//! To solve this, `cps-signal` provides polymorphic capability traits and free
//! helper functions:
//!
//! - **Break capabilities**: [`CanBreak0`] through [`CanBreak9`], with free
//!   functions [`break_0`] through [`break_9`].
//! - **Continue capabilities**: [`CanContinue0`] through [`CanContinue8`], with
//!   free functions [`continue_0`] through [`continue_8`].
//! - **Return capability**: [`CanReturn`], with free function [`return_`].
//!
//! Any continuation can invoke [`break_0(val)`] or [`return_(val)`] polymorphically;
//! type inference resolves the result to whatever signal level is required by
//! the enclosing scope (`L0Signal`, `L1Signal`, `L2Signal`, etc.).
//!
//! # Examples
//!
//! ## Concrete Signal Matching
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
//! ## Polymorphic Signal Construction
//!
//! ```
//! use cps_signal::{break_0, return_, CanBreak0, CanReturn, L0Signal, L1Signal, L2Signal, L0SignalNoBreakLocal};
//!
//! // This function can emit into any signal level that can break to loop 0:
//! fn emit_break<C: CanBreak0<B0 = i32>>(value: i32) -> C {
//!     break_0(value)
//! }
//!
//! let at_l0: L0Signal<i32> = emit_break(10);
//! let at_l1: L1Signal<(), (), i32> = emit_break(20);
//! let at_l2: L2Signal<(), (), (), (), i32> = emit_break(30);
//!
//! // Similarly, return can target any level up to the function boundary:
//! fn emit_return<C: CanReturn<R = &'static str>>(msg: &'static str) -> C {
//!     return_(msg)
//! }
//!
//! let ret_l0: L0SignalNoBreakLocal<&'static str> = emit_return("done");
//! let ret_l1: L1Signal<(), (), core::convert::Infallible, &'static str> = emit_return("done");
//! ```

#![allow(non_camel_case_types)]
#![no_std]

mod loop_;

pub use loop_::*;
