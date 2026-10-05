//! # `colorless-std`
//!
//! The standard library for colored Rust execution.
//!
//! Mirrors standard library abstractions across execution contexts:
//! - [`ops`]: Function traits ([`ops::ColoredFn`]) and closure combinators.
//! - [`iter`]: Colored iterators ([`iter::ColoredIterator`]) that evaluate synchronously
//!   in `Identity` and asynchronously as unboxed streams in `Async`.
//! - [`task`]: Cooperative scheduling and task primitives.

#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

pub mod iter {
    //! Colored iterator abstractions and adapters.
}

pub mod ops {
    //! Function traits, closures, and higher-order colored combinators.
}

pub mod task {
    //! Cooperative scheduling across execution colors.
}
