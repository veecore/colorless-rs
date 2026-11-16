//! # `colorless-std`
//!
//! The standard library for colored Rust execution.
//!
//! Mirrors standard library abstractions across execution contexts:
//! - [`function`]: Function traits ([`function::ColoredFn`]) and closure combinators.
//! - [`iter`]: Colored iterators ([`iter::ColoredIterator`]) that evaluate synchronously
//!   in `Identity` and asynchronously as unboxed streams in `Async`.
//! - [`io`]: Color-generic I/O primitives.

#![no_std]
#![cfg_attr(feature = "nightly", feature(async_iterator))]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

pub mod function;
#[cfg(feature = "std")]
pub mod io;
pub mod iter;
