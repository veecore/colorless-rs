use crate::Colored;
use crate::colored::internal::{ColoredInternal, Present};
use core::pin::Pin;
use core::task::{Context, Poll};

/// Conversion into a [`Colored`] computation.
///
/// By implementing `IntoColored<C>` for a type, you define how it converts
/// into a color-generic computation under context `C`.
///
/// # `.resolve` desugaring
///
/// The `.resolve` call access on a [`Colored<C>`] desugars into a call to
/// `IntoColored::into_colored` before the resulting computation is driven by
/// the active color.  `IntoColored` is implemented for all `T: Colored<C>`,
/// meaning `into_colored` is available on every computation and the extra
/// call is a no-op identity conversion.
///
/// ```rust,ignore
/// use colorless_core::IntoColored;
///
/// // Inside a #[colorless] function, every .resolve:
/// let value = step().resolve;
///
/// // is equivalent to:
/// let colored = step().into_colored();  // IntoColored::into_colored
/// let value   = /* color drives colored to completion */;
/// ```
///
/// # Delayed builders
///
/// The current most powerful use of `IntoColored<C>` is building sync
/// types whose resolution is deferred
///
/// ```rust
/// use colorless_core::{Color, Colored, Identity, IntoColored, IdentityExt, Ready};
/// # use colorless_core::test_util::block_on_test;
///
/// /// A builder that fetches a remotely-computed value with optional retry.
/// struct FetchBuilder {
///     url: &'static str,
///     retries: u32,
/// }
///
/// impl FetchBuilder {
///     pub fn new(url: &'static str) -> Self {
///         Self { url, retries: 0 }
///     }
///
///     /// Retry up to `n` times on failure.
///     pub fn retries(mut self, n: u32) -> Self {
///         self.retries = n;
///         self
///     }
/// }
///
/// impl IntoColored<Identity> for FetchBuilder {
///     type Output = &'static str;
///
///     fn into_colored(self) -> impl Colored<Identity, Output = &'static str> {
///         // Build the computation only when resolved.
///         Identity::ready(self.url)
///     }
/// }
///
/// let result = FetchBuilder::new("https://example.com")
///     .retries(3)
///     .into_colored()   // converts to Colored<Identity>
///     .sync_();         // drives it synchronously
///
/// assert_eq!(result, "https://example.com");
/// ```
///
/// This mirrors the async-builder pattern enabled by `IntoFuture` in the
/// standard library, generalized across all [`Color`](crate::Color)s.
///
/// # Usage in trait bounds
///
/// Bounding on `IntoColored<C>` lets a function accept both a plain
/// `Colored<C>` value *and* a builder that converts into one, without
/// requiring callers to call `into_colored` explicitly:
///
/// ```rust
/// use colorless_core::{Color, Colored, Identity, IntoColored, IdentityExt};
///
/// /// Runs a computation and maps its output to a string.
/// fn run_to_string<C, I>(computation: I) -> impl Colored<C, Output = String>
/// where
///     C: Color,
///     I: IntoColored<C>,
///     I::Output: core::fmt::Debug,
/// {
///     // The caller passes either a Colored<C> or a builder; both work.
///     let colored = computation.into_colored();
///     C::map(colored, |v| format!("{v:?}"))
/// }
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid Colored computation in execution context `{C}`",
    label = "expected Colored<{C}> computation",
    note = "`{Self}` must be Colored over `{C}` or must implement `IntoColored<C>` to be resolved"
)]
pub trait IntoColored<C> {
    /// The value produced when the converted computation completes.
    type Output;

    /// Converts `self` into a [`Colored<C>`] computation.
    ///
    /// For types that already implement `Colored<C>` this is a no-op identity
    /// conversion provided by the blanket impl. For builder types it defers
    /// construction of the computation until this method is called.
    fn into_colored(self) -> impl Colored<C, Output = Self::Output>;
}

impl<C, D: Colored<C>> IntoColored<C> for D {
    type Output = D::Output;

    fn into_colored(self) -> impl Colored<C, Output = Self::Output> {
        self
    }
}

/// A transparent wrapper that exposes the inner computation through a
/// named field, mirroring `.await` syntax for color-generic code.
///
/// The `#[colorless]` macro wraps every yield-point result in `Resolvable`
/// when `autocomplete` is enabled so that call sites can write `.resolve`
/// with IDE aid to extract the value:
///
/// `Resolvable<D>` is `#[repr(transparent)]` over `D`, so it carries
/// **zero runtime cost** compared to using `D` directly.
///
/// # Implementing `IntoColored<C>` for a builder
///
/// If your builder returns a `Resolvable` from `into_colored`, the
/// `.resolve` syntax works on the builder's return value at a yield site
/// as naturally as it does on any `Colored<C>` value.
#[must_use = "Colored computations do nothing unless executed in their traits context"]
#[repr(transparent)]
pub struct Resolvable<D> {
    /// The inner [`Colored<C>`] computation.
    pub resolve: D,
}

unsafe impl<C, D: ColoredInternal<C>> ColoredInternal<C> for Resolvable<D> {
    type OutputInternal = D::OutputInternal;

    #[inline(always)]
    unsafe fn poll_colored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<<C as Present>::Value<Self::OutputInternal>>
    where
        C: Present,
    {
        unsafe {
            let resolve = self.map_unchecked_mut(|s| &mut s.resolve);
            resolve.poll_colored(cx)
        }
    }
    #[inline(always)]
    unsafe fn finalize_ready(self: Pin<&mut Self>) {
        unsafe {
            let resolve = self.map_unchecked_mut(|s| &mut s.resolve);
            resolve.finalize_ready()
        }
    }
    #[inline(always)]
    fn compute_sync(self) -> <C as Present>::Value<Self::OutputInternal>
    where
        C: Present,
        Self: Sized,
    {
        self.resolve.compute_sync()
    }
}

impl<C, D: Colored<C>> Colored<C> for Resolvable<D> {
    type Output = D::Output;
}
