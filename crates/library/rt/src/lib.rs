//! Synthetic runtime items emitted by `colorless-codegen`.
//! Not intended for direct human consumption.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

pub mod borrow;
pub mod branch;
pub mod inert;
pub mod loop_;

pub use borrow::*;
pub use branch::*;
pub use inert::*;
pub use loop_::*;

// Re-export the runtime capability traits so codegen only needs `::colorless_rt`
pub use colorless_core::*;
pub use cps_signal::*;
pub use cps_try::*;

// Alias to allow `$crate::__compiler::...` macro expansions to resolve uniformly
#[doc(hidden)]
pub use crate as __compiler;
