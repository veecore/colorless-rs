//! # Colorless
//!
//! Zero-cost, unboxed abstractions for writing logic that can execute
//! seamlessly across multiple execution contexts (such as synchronous `Identity`
//! and asynchronous `Async` runtimes) without code duplication.

#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

// Re-export core primitives
pub use colorless_core::*;

// Re-export standard library abstractions when enabled
#[cfg(feature = "std")]
pub use colorless_std::{iter, ops, task};

// Re-export macros
pub use colorless_macros::{color, colored, colorless};
