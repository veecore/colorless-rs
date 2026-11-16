//! A stable implementation of Rust's `Try` and `FromResidual`.
//!
//! This crate provides the trait machinery necessary to desugar the `?` operator
//! on stable Rust across [`Result`], [`Option`], [`ControlFlow`], [`Poll`], and layered
//! types (e.g. `Poll<Result<T, E>>`, `Poll<Option<Result<T, E>>>`).
//!
//! ### The `?` Desugaring Pattern
//! ```rust
//! # fn func() -> Result<(), Box<dyn std::error::Error>> {
//! # let expr = Result::<(), &'static str>::Err("failed");
//! match cps_try::branch(expr) {
//!     std::ops::ControlFlow::Continue(val) => val,
//!     std::ops::ControlFlow::Break(residual) => return cps_try::from_residual(residual),
//! };
//! # Ok(())
//! # }
//! ```
//! Because `return from_residual(residual)` is evaluated in return position,
//! Rust's standard type inference resolves the target return type **automatically**
//! without requiring any explicit type annotations.
#![no_std]

#[cfg(test)]
extern crate std;

use core::ops::ControlFlow;
use core::task::Poll;

// ---------------------------------------------------------------------------
// Residual Types
// ---------------------------------------------------------------------------

/// The residual type representing a failed [`Result`].
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResultResidual<E>(pub E);

/// The residual type representing a [`None`] value for [`Option`].
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OptionResidual;

/// The residual type representing an early break for [`ControlFlow`].
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ControlFlowResidual<B>(pub B);

// ---------------------------------------------------------------------------
// Core Traits
// ---------------------------------------------------------------------------

/// Splitting an expression into a success value or an early-exit residual.
pub trait Try: Sized {
    /// The type of the value produced when execution continues successfully.
    type Output;

    /// The residual type passed to [`FromResidual::from_residual`] on early exit.
    type Residual;

    /// Splits `self` into either a continuation output or an early-exit residual.
    ///
    /// Following RFC 3058, this returns [`ControlFlow`] which has identical ABI
    /// and zero-cost machine representation to [`Result`] while cleanly modeling
    /// early returns for non-error types like [`Option`] and [`Poll`].
    fn branch(self) -> ControlFlow<Self::Residual, Self::Output>;
}

/// Constructing a return type from an early-exit residual.
pub trait FromResidual<Residual = <Self as Try>::Residual> {
    /// Constructs `Self` from the given residual.
    fn from_residual(residual: Residual) -> Self;
}

// ---------------------------------------------------------------------------
// Helper Functions
// ---------------------------------------------------------------------------

/// Branch a `Try` expression into [`ControlFlow`].
#[inline(always)]
pub fn branch<T: Try>(val: T) -> ControlFlow<T::Residual, T::Output> {
    val.branch()
}

/// Convert an early-exit residual into the target return type.
///
/// Thanks to Rust's return-type inference, calling `return from_residual(residual)`
/// automatically infers `Target` from the enclosing function's signature.
#[inline(always)]
pub fn from_residual<Target: FromResidual<R>, R>(residual: R) -> Target {
    Target::from_residual(residual)
}

// ---------------------------------------------------------------------------
// Implementations: Result
// ---------------------------------------------------------------------------

impl<T, E> Try for Result<T, E> {
    type Output = T;
    type Residual = ResultResidual<E>;

    #[inline(always)]
    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        match self {
            Ok(v) => ControlFlow::Continue(v),
            Err(e) => ControlFlow::Break(ResultResidual(e)),
        }
    }
}

impl<T, E, F: From<E>> FromResidual<ResultResidual<E>> for Result<T, F> {
    #[inline(always)]
    fn from_residual(residual: ResultResidual<E>) -> Self {
        Err(From::from(residual.0))
    }
}

// ---------------------------------------------------------------------------
// Implementations: Option
// ---------------------------------------------------------------------------

impl<T> Try for Option<T> {
    type Output = T;
    type Residual = OptionResidual;

    #[inline(always)]
    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        match self {
            Some(v) => ControlFlow::Continue(v),
            None => ControlFlow::Break(OptionResidual),
        }
    }
}

impl<T> FromResidual<OptionResidual> for Option<T> {
    #[inline(always)]
    fn from_residual(_residual: OptionResidual) -> Self {
        None
    }
}

// ---------------------------------------------------------------------------
// Implementations: ControlFlow
// ---------------------------------------------------------------------------

impl<B, C> Try for ControlFlow<B, C> {
    type Output = C;
    type Residual = ControlFlowResidual<B>;

    #[inline(always)]
    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        match self {
            ControlFlow::Continue(c) => ControlFlow::Continue(c),
            ControlFlow::Break(b) => ControlFlow::Break(ControlFlowResidual(b)),
        }
    }
}

impl<B, C, B2: From<B>> FromResidual<ControlFlowResidual<B>> for ControlFlow<B2, C> {
    #[inline(always)]
    fn from_residual(residual: ControlFlowResidual<B>) -> Self {
        ControlFlow::Break(From::from(residual.0))
    }
}

// ---------------------------------------------------------------------------
// Implementations: Poll and Layered Combinations
// ---------------------------------------------------------------------------

impl<T, E> Try for Poll<Result<T, E>> {
    type Output = Poll<T>;
    type Residual = ResultResidual<E>;

    #[inline(always)]
    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        match self {
            Poll::Ready(Ok(v)) => ControlFlow::Continue(Poll::Ready(v)),
            Poll::Ready(Err(e)) => ControlFlow::Break(ResultResidual(e)),
            Poll::Pending => ControlFlow::Continue(Poll::Pending),
        }
    }
}

impl<T, E, F: From<E>> FromResidual<ResultResidual<E>> for Poll<Result<T, F>> {
    #[inline(always)]
    fn from_residual(residual: ResultResidual<E>) -> Self {
        Poll::Ready(Err(From::from(residual.0)))
    }
}

impl<T> Try for Poll<Option<T>> {
    type Output = Poll<T>;
    type Residual = OptionResidual;

    #[inline(always)]
    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        match self {
            Poll::Ready(Some(v)) => ControlFlow::Continue(Poll::Ready(v)),
            Poll::Ready(None) => ControlFlow::Break(OptionResidual),
            Poll::Pending => ControlFlow::Continue(Poll::Pending),
        }
    }
}

impl<T> FromResidual<OptionResidual> for Poll<Option<T>> {
    #[inline(always)]
    fn from_residual(_residual: OptionResidual) -> Self {
        Poll::Ready(None)
    }
}

// Interop: using `result?` inside a function returning `Option<Result<T, F>>`
impl<T, E, F: From<E>> FromResidual<ResultResidual<E>> for Option<Result<T, F>> {
    #[inline(always)]
    fn from_residual(residual: ResultResidual<E>) -> Self {
        Some(Err(From::from(residual.0)))
    }
}

// Interop: using `result?` inside a function returning `Poll<Option<Result<T, F>>>`
impl<T, E, F: From<E>> FromResidual<ResultResidual<E>> for Poll<Option<Result<T, F>>> {
    #[inline(always)]
    fn from_residual(residual: ResultResidual<E>) -> Self {
        Poll::Ready(Some(Err(From::from(residual.0))))
    }
}

// ---------------------------------------------------------------------------
// Convenience Desugaring Macro
// ---------------------------------------------------------------------------

/// Desugars the `?` operator on stable Rust.
#[macro_export]
macro_rules! try_op {
    ($expr:expr) => {
        match $crate::branch($expr) {
            ::core::ops::ControlFlow::Continue(val) => val,
            ::core::ops::ControlFlow::Break(residual) => {
                return $crate::from_residual(residual);
            }
        }
    };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use core::error::Error;
    use std::boxed::Box;
    use std::string::{String, ToString};

    fn parse_int(s: &str) -> Result<i32, core::num::ParseIntError> {
        s.parse::<i32>()
    }

    #[test]
    fn test_result_type_inference_with_error_conversion() {
        fn run() -> Result<i32, Box<dyn Error>> {
            let x = try_op!(parse_int("42"));
            Ok(x * 2)
        }

        assert_eq!(run().unwrap(), 84);

        fn run_err() -> Result<i32, Box<dyn Error>> {
            let _ = try_op!(parse_int("not a number"));
            Ok(0)
        }

        assert!(run_err().is_err());
    }

    #[test]
    fn test_option_desugaring() {
        fn find_even(v: Option<i32>) -> Option<i32> {
            let val = try_op!(v);
            if val % 2 == 0 { Some(val) } else { None }
        }

        assert_eq!(find_even(Some(4)), Some(4));
        assert_eq!(find_even(Some(3)), None);
        assert_eq!(find_even(None), None);
    }

    #[test]
    fn test_control_flow_desugaring() {
        fn search(step: i32) -> ControlFlow<&'static str, i32> {
            if step > 10 {
                ControlFlow::Break("limit reached")
            } else {
                ControlFlow::Continue(step + 1)
            }
        }

        fn run(start: i32) -> ControlFlow<&'static str, i32> {
            let a = try_op!(search(start));
            let b = try_op!(search(a));
            ControlFlow::Continue(b)
        }

        assert_eq!(run(0), ControlFlow::Continue(2));
        assert_eq!(run(10), ControlFlow::Break("limit reached"));
    }

    #[test]
    fn test_poll_result_desugaring() {
        fn poll_fn(ready: bool) -> Poll<Result<i32, &'static str>> {
            if ready {
                Poll::Ready(Ok(10))
            } else {
                Poll::Pending
            }
        }

        fn run_poll(ready: bool) -> Poll<Result<i32, String>> {
            // `try_op!` unwraps `Poll::Ready(Ok(10))` into `Poll::Ready(10)` or handles Pending
            let poll_val = try_op!(poll_fn(ready));
            match poll_val {
                Poll::Ready(v) => Poll::Ready(Ok(v * 2)),
                Poll::Pending => Poll::Pending,
            }
        }

        assert_eq!(run_poll(true), Poll::Ready(Ok(20)));
        assert_eq!(run_poll(false), Poll::Pending);
    }

    #[test]
    fn test_result_inside_poll_result_interop() {
        // Here we test using `?` on a plain `Result` inside a function returning `Poll<Result>`!
        fn poll_with_sync_check(input: Result<i32, &'static str>) -> Poll<Result<i32, String>> {
            let val = try_op!(input);
            Poll::Ready(Ok(val + 1))
        }

        assert_eq!(poll_with_sync_check(Ok(5)), Poll::Ready(Ok(6)));
        assert_eq!(
            poll_with_sync_check(Err("fail")),
            Poll::Ready(Err("fail".to_string()))
        );
    }
}
