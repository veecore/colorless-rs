//! # `colorless-core`
//!
//! Foundational, zero-cost, unboxed monad primitives and capability traits
//! for color-agnostic Rust execution.
//!
//! This crate is `#![no_std]` by default and performs zero heap allocations.

#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

// Re-export core control signals and try machinery
pub use cps_signal as signal;
pub use cps_try as r#try;
