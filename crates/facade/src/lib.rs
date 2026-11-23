//! # Colorless
//!
//! Zero-cost, unboxed abstractions for writing logic that can execute
//! seamlessly across multiple execution contexts (such as synchronous `Identity`
//! and asynchronous `Async` runtimes) without code duplication.

#![no_std]

#[doc(hidden)]
pub mod __compiler;

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

// Re-export core primitives
#[doc(hidden)] // no one likes you
pub use colorless_core::*;
pub use colorless_rt::{
    __borrow, __pin, __rb, __unborrow, __unpin, borrow, keep_alive, rb, unborrow,
};

// Re-export standard library abstractions when enabled
#[cfg(feature = "std")]
pub use colorless_std::{function, io, iter};

// Re-export macros
pub use colorless_macros::{color, colorless};
