//! # Colorless
//!
//! A zero-cost foundation for solving the function coloring problem in Rust.
//!
//! ---
//!
//! ## The Problem
//!
//! In Rust, as in many languages with first-class asynchronous programming, functions are
//! divided into "colors": synchronous functions execute directly on the current call stack,
//! while asynchronous functions construct state machines that yield to an external executor.
//!
//! For application developers, this distinction is natural. But for generic library authors—those
//! writing parsers, serializers, protocol codecs, data-structure traversals, or business pipelines—it
//! creates a painful dilemma:
//!
//! 1. **Duplicate implementations**: Maintaining identical business logic in both a `sync` module
//!    and an `async` module leads to divergence, doubled maintenance, and subtle behavioral drift.
//! 2. **Conditional compilation (`maybe-async`)**: Macro attributes that rewrite `async fn` to `fn`
//!    at compile time lock the entire crate into one mode per build target. A library cannot
//!    expose both synchronous and asynchronous APIs to the same downstream consumer.
//! 3. **Blocking on async**: Forcing synchronous callers to drive futures via `block_on` introduces
//!    allocator, thread-synchronization, and runtime overhead into contexts that should be pure machine code.
//!
//! `colorless` solves this by allowing library authors to write logic **once**, generic over an
//! execution context `C: Color`. The resulting code compiles down to direct, unboxed machine code
//! when called synchronously, and into compact, polled state machines when called asynchronously.
//!
//! ---
//!
//! ## Core Architecture: The `Color` and `Colored<C>` Duality
//!
//! The framework rests on two complementary abstractions:
//!
//! ```text
//! ┌────────────────────────────────────────────────────────┐
//! │                 The Colorless Contract                 │
//! ├──────────────────────────┬─────────────────────────────┤
//! │  Color (The Runtime)     │  Colored<C> (The Step)      │
//! ├──────────────────────────┼─────────────────────────────┤
//! │  Defines execution rules │  An unboxed computation     │
//! │  - Identity (Sync)       │  bound to execution context │
//! │  - Async (Futures)       │  `C`, yielding an output    │
//! │  - Optional / Fallible   │  upon resolution.           │
//! └──────────────────────────┴─────────────────────────────┘
//! ```
//!
//! ### The "Associated Trait" Approximation
//!
//! An ideal language feature for function coloring would be an *associated trait*: a capability where
//! each runtime could define its own return type and combinators directly.
//!
//! Because Rust does not support this imaginary associated traits, and because generic associated types (GATs)
//! would force async to box, `colorless` models pending steps as `impl Colored<C, Output = T>`.
//!
//! The trait is parameterized by `C`, allowing each color to enforce uniqueness and provide specialized
//! unboxed representations. Downstream callers only see the public `Colored<C>` interface, while sealed
//! internal traits allow each runtime to extract and drive its concrete representation without heap boxing
//! or dynamic dispatch.
//!
//! ---
//!
//! ## Zero-Cost Monomorphization
//!
//! A primary design requirement is that synchronous code must incur **zero overhead**:
//!
//! - In [`Identity`], combinators inline directly. Step transitions compile away into ordinary sequential
//!   instructions, and loops become native CPU jumps. No generator state is allocated, and no poll functions
//!   are registered.
//! - In [`Async`], combinators lower into compact, unboxed enum state machines driven by standard Rust
//!   executors (Tokio, smol, etc.). Memory layout grows linearly rather than exponentially.
#![no_std]
#![warn(missing_docs)]
// #![warn(clippy::undocumented_unsafe_blocks)]

#[cfg(any(feature = "alloc", test))]
extern crate alloc;
#[cfg(any(feature = "std", test))]
extern crate std;

mod capabilities;
mod colored;
mod colorize;
mod colors;
mod into_colored;

pub use capabilities::*;
pub use colored::*;
pub use colorize::*;
pub use colors::*;
pub use into_colored::*;

/// Creates a computation that immediately yields `value`.
#[inline]
pub fn ready<C: Color, T>(a: T) -> impl Colored<C, Output = T> {
    C::ready(a)
}

// Support items used exclusively by rustdoc examples.
#[doc(hidden)]
pub mod test_util {
    use crate::{Color, Colored};
    use core::pin::Pin;
    use core::task::{Context, Poll};

    // for purposely not send futures
    #[inline(never)]
    #[doc(hidden)]
    pub fn block_on_local_test<F: Future>(fut: F) -> F::Output {
        use core::task::{Context, Poll, Waker};
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);
        let mut pinned = core::pin::pin!(fut);
        loop {
            if let Poll::Ready(v) = pinned.as_mut().poll(&mut cx) {
                return v;
            }
        }
    }

    /// Minimal synchronous executor for polling a future to completion (used in tests and benchmarks).
    #[inline(always)]
    #[doc(hidden)]
    // The Send isn't needed but helps catch derailing early
    pub fn block_on_test<F: Future + Send + Sync>(fut: F) -> F::Output {
        block_on_local_test(fut)
    }

    pub fn fetch<C: Color, T>(a: T) -> impl Colored<C, Output = T> {
        C::ready(a)
    }

    pub fn fetch_from_remote<C: Color, T: Default>() -> impl Colored<C, Output = T> {
        C::ready(T::default())
    }

    pub fn fetch_profile<C: Color>(_id: u64) -> impl Colored<C, Output = UserProfile> {
        C::ready(UserProfile)
    }

    pub fn fetch_user<C: Color>() -> impl Colored<C, Output = User> {
        C::ready(User { id: 2 })
    }

    #[inline(never)]
    #[doc(hidden)]
    pub fn poll_once<F>(mut future: Pin<&mut F>) -> Poll<F::Output>
    where
        F: Future,
    {
        let waker = core::task::Waker::noop();
        let mut cx = Context::from_waker(waker);
        future.as_mut().poll(&mut cx)
    }

    #[derive(Debug)]
    pub struct User {
        pub id: u64,
    }

    pub struct UserProfile;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::marker::PhantomData;
    use std::prelude::v1::Box;

    #[test]
    fn impl_color() {
        fn assert_color<T: Color>() {}
        assert_color::<Async>();
        assert_color::<FallibleAsync<All>>();
        assert_color::<FallibleAsync<&All>>();
        #[cfg(feature = "rainbow")]
        assert_color::<FallibleOptionAsync<All>>();
        #[cfg(feature = "rainbow")]
        assert_color::<FallibleOptionAsync<&All>>();
        assert_color::<OptionalAsync>();
        #[cfg(feature = "rainbow")]
        assert_color::<OptionalResultAsync<All>>();
        #[cfg(feature = "rainbow")]
        assert_color::<OptionalResultAsync<&All>>();

        assert_color::<Fallible<All>>();
        assert_color::<Fallible<&All>>();
        #[cfg(feature = "rainbow")]
        assert_color::<FallibleOption<All>>();
        #[cfg(feature = "rainbow")]
        assert_color::<FallibleOption<&All>>();
        assert_color::<Identity>();
        assert_color::<Optional>();
        #[cfg(feature = "rainbow")]
        assert_color::<OptionalResult<All>>();
        #[cfg(feature = "rainbow")]
        assert_color::<OptionalResult<&All>>();

        fn _non_static<E>(_err: &E) -> impl Color {
            Fallible::<&E>(PhantomData)
        }
        struct All;
    }

    #[test]
    fn assert_dyn_safe() {
        fn _assert_dyn<C, T>(_c: Box<dyn Colored<C, Output = T, OutputInternal = T>>) {}
    }
}
