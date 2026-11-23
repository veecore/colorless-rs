use crate::colored::internal::{ColoredInternal, Present};
use core::marker::PhantomData;
use core::pin::Pin;
use core::task::{Context, Poll};

/// A color-generic computation that produces an `Output` value within
/// execution context `C`.
///
/// A type that implements `Colored<C>` defines how it participates
/// in the execution model of a chosen [`Color`](crate::Color).  Functions written
/// generically over `C: Color` return `impl Colored<C, Output = T>`.
/// Depending on the caller's choice of `C`, the same code either runs
/// eagerly on the current call stack or becomes a polled asynchronous
/// state machine — with **zero boxing and zero dynamic dispatch**.
///
/// # `.resolve` desugaring
///
/// Inside a function lowered by the `#[colorless]` procedural macro, the
/// `.resolve` field access is the color-generic equivalent of `.await`.
/// The macro desugars each yield point into a call to
/// [`IntoColored::into_colored`](crate::IntoColored::into_colored)
/// followed by color-generic execution:
///
/// ```rust,ignore
/// // Source (direct style, inside #[colorless]):
/// let value = step().resolve;
///
/// // Desugared (conceptually):
/// let value = C::map(step().into_colored(), |v| v);
/// ```
///
/// `IntoColored` is implemented for all `T: Colored<C>`, so `.resolve`
/// is available on every computation without an extra conversion call.
///
/// # Implementing `Colored<C>`
///
/// `Colored<C>` cannot be implemented outside this crate yet — the [`Color`](crate::Color)
/// combinators (`ready`, `map`, `and_then`, `wrap_flatten`, …) build all
/// needed computations from closures.
///
/// When wrapping a foreign type (for example, an existing `impl Future`)
/// use [`Colorize`](crate::Colorize) instead:
///
/// ```rust
/// use colorless_core::{Color, Colored, Async, AsyncExt, Colorize};
///
/// // Any Future<Output = T> becomes Colored<Async, Output = T> via Colorize.
/// let colored = <_ as Colorize<Async>>::colorize(core::future::ready(42_u32));
/// # let _ = colored.async_();
/// ```
///
/// For a handwritten synchronous implementation use [`Identity`](crate::Identity) as a
/// concrete `C` to verify the logic compiles, then make it generic:
///
/// ```rust
/// use colorless_core::{Color, Colored, Identity, IdentityExt};
///
/// fn double<C: Color>(x: i32) -> impl Colored<C, Output = i32> {
///     C::map(C::ready(x), |v| v * 2)
/// }
///
/// assert_eq!(double::<Identity>(21).sync_(), 42);
/// ```
///
/// # Choosing a `Color`
///
/// The caller selects the execution model by choosing `C` and calling the
/// appropriate extractor:
///
/// | `C` | Extractor | Return type |
/// |---|---|---|
/// | [`Identity`](crate::Identity) | `.sync_()` | `T` |
/// | [`Optional`](crate::Optional) | `.option_()` | `Option<T>` |
/// | [`Fallible<E>`](crate::Fallible) | `.result_()` | `Result<T, E>` |
/// | [`Async`](crate::Async) | `.async_().await` | `T` |
///
/// # Usage in trait bounds
///
/// Bounding on `Colored<C>` lets a function accept any computation that
/// resolves under context `C`:
///
/// ```rust
/// use colorless_core::{Color, Colored, Identity, IdentityExt, Ready};
///
/// fn run_and_double<C, D>(computation: D) -> impl Colored<C, Output = i64>
/// where
///     C: Color,
///     D: Colored<C, Output = i32>,
/// {
///     C::map(computation, |v| v as i64 * 2)
/// }
///
/// assert_eq!(run_and_double::<Identity, _>(Identity::ready(21_i32)).sync_(), 42);
/// ```
#[must_use = "Colored computations do nothing unless executed in their traits context"]
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid Colored computation in execution context `{C}`",
    label = "expected Colored<{C}> computation",
    note = "pure values can be lifted into `{C}` using `{C}::ready(...)`"/**/
)]
pub trait Colored<C>: ColoredInternal<C, OutputInternal = Self::Output> {
    /// The value produced when this computation completes.
    type Output;
}

/// Stand-in `Colored<C>` implementation used in tests and as a compile-time
/// placeholder where a concrete type is not yet available.
pub struct PlaceHolder<C, T> {
    void: Void,
    color: PhantomData<C>,
    data: PhantomData<T>,
}

enum Void {}

/// Crate-internal sealed abstractions for zero-cost unboxed state machines.
pub(crate) mod internal {
    use core::pin::Pin;
    use core::task::{Context, Poll};

    /// The sealed internal protocol used to drive zero-cost unboxed state machines.
    ///
    /// Generic functions only see `Colored<C>`, but the built-in colors use this
    /// trait to extract their concrete representation without boxing. Asynchronous
    /// implementations use a destructive polling protocol: completion may consume
    /// fields in place, so a completed value is not necessarily valid for ordinary
    /// Rust drop glue.
    ///
    /// # Safety
    ///
    /// An asynchronous implementation must obey all the following rules:
    ///
    /// - Returning [`Poll::Pending`] leaves `self` live, pinned in place, and valid
    ///   for another call to [`poll_colored`](Self::poll_colored) or for ordinary
    ///   drop.
    /// - Returning [`Poll::Ready`] transfers the remaining cleanup obligation to
    ///   [`finalize_ready`](Self::finalize_ready). The caller must invoke it exactly
    ///   once, immediately and at the same pinned address. Until then, the caller
    ///   must neither poll nor ordinarily drop `self`.
    /// - Returning normally or unwinding from `finalize_ready` leaves `self`
    ///   logically dead: all still-live state has been dropped in place.
    /// - Unwinding from `poll_colored` also leaves `self` logically dead. Every
    ///   field that was live when polling began, including fields consumed during
    ///   that poll, must be dropped exactly once.
    ///
    /// These rules let readiness and panic cleanup travel through the call stack
    /// instead of requiring a completed tag in every composed state machine.
    pub unsafe trait ColoredInternal<C> {
        /// The internal output type.
        type OutputInternal;

        /// Polls an asynchronous computation in place.
        ///
        /// # Safety
        ///
        /// `self` must be live and must not previously have returned `Ready` or
        /// panicked from this method. The caller must uphold the completion protocol
        /// documented on [`ColoredInternal`].
        unsafe fn poll_colored(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<<C as Present>::Value<Self::OutputInternal>>
        where
            C: Present,
        {
            unreachable!("Sync computations cannot be awaited asynchronously")
        }

        /// Finishes in-place destruction after `poll_colored` returns `Ready`.
        ///
        /// # Safety
        ///
        /// The immediately preceding poll of this same pinned value must have
        /// returned `Ready`, and this method must not have been called for that
        /// completion before. After this call begins, `self` is logically dead even
        /// if destruction unwinds.
        unsafe fn finalize_ready(self: Pin<&mut Self>) {
            panic!("finalize_ready must be implemented for async computations");
        }

        /// Computes the synchronous result directly. By default, this is unreachable for asynchronous colors.
        fn compute_sync(self) -> <C as Present>::Value<Self::OutputInternal>
        where
            C: Present,
            Self: Sized,
        {
            unreachable!("Async computations must be resolved asynchronously")
        }
    }

    /// The presentation layer for a color's resolved output.
    ///
    /// Every color `C: Present` declares `type Value<O>` — the concrete wrapper
    /// type around an output `O` after resolution:
    ///
    /// | Color | `Value<O>` |
    /// |---|---|
    /// | `Identity` | `ReadyValue<O>` (newtype over `O`) |
    /// | `Optional` | `OptionalValue<O>` (newtype over `Option<O>`) |
    /// | `Fallible<E>` | `FallibleValue<O, E>` (newtype over `Result<O,E>`) |
    /// | `Async` | `O` (bare `T`; the future resolves to it directly) |
    /// | `FallibleAsync<E>` | `Result<O, E>` |
    ///
    /// `Present` is sealed inside `colored::internal`; downstream code should
    /// never implement it. Use the `Ext` traits (`IdentityExt`, `FallibleExt`,
    /// …) to extract the value instead.
    pub trait Present {
        type Value<O>;
        // we could prolly leverage a trait like this to totally avoid multimethods
        // fn execute_in(a: <C as ColorInternal>::Arg<Self) -> <C as ColorInternal>::Ret
        // fn finalize_in(a: <C as ColorInternal>::FinalizeArg<Self>)
        // and hopefully make this trait pub

        #[cfg(test)]
        fn from<O>(o: O) -> Self::Value<O>;
    }
}

unsafe impl<C, T> ColoredInternal<C> for PlaceHolder<C, T> {
    type OutputInternal = T;

    unsafe fn poll_colored(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<<C as Present>::Value<Self::OutputInternal>>
    where
        C: Present,
    {
        match self.void {}
    }

    unsafe fn finalize_ready(self: Pin<&mut Self>) {
        todo!()
    }

    fn compute_sync(self) -> <C as Present>::Value<Self::OutputInternal>
    where
        C: Present,
        Self: Sized,
    {
        match self.void {}
    }
}

impl<C, T> Colored<C> for PlaceHolder<C, T> {
    type Output = T;
}
