//! # `colorless-core`
//!
//! Foundational, zero-cost, unboxed monad primitives and capability traits
//! for color-agnostic Rust execution.

#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

pub mod callback;
pub mod colors;
pub mod erase;
pub mod local;
pub mod loop_;
pub mod one_of;

// Re-export core control signals and try machinery
pub use cps_signal as signal;
pub use cps_try as r#try;
