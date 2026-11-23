use crate::Colored;
use crate::capabilities::callbacks::{ColoredThunkOnce, ThunkOnce};

/// Context capability for lifting a pure value into a [`Colored`] computation.
pub trait Ready: Sized {
    /// Creates a computation that immediately yields `value`.
    fn ready<T>(value: T) -> impl Colored<Self, Output = T>;

    #[inline(always)]
    #[allow(missing_docs)] // todo
    fn ready_fn<F: ThunkOnce>(func: F) -> impl Colored<Self, Output = F::Value> {
        Self::ready(func())
    }
}

/// When writing code generic across execution models, a fundamental question might be:
/// **when does a piece of code actually run?**
///
/// In pure synchronous execution, every statement runs eagerly the moment it is encountered.
/// In asynchronous execution, construction of a future and its eventual execution are separated
/// in time: building a future merely packages state, while execution happens when the future is polled.
///
/// `colorless_core` formalizes this through two distinct degrees of deferral:
///
/// | Capability | Method | Synchronous Timing ([`Identity`]) | Asynchronous Timing ([`Async`]) |
/// | :--- | :--- | :--- | :--- |
/// | **[`Ready`]** | `ready(val)` | Immediate pure value | Resolved immediately on first poll |
/// | **[`Wrap`]** | `wrap_flatten(f)` | **Eager**: closure runs immediately | **Deferred**: closure runs on first poll |
///
/// ---
///
/// ### `Wrap`: The Native Execution Boundary
///
/// [`Wrap`] mirrors the native execution model of the underlying runtime:
///
/// ```rust,ignore
/// C::wrap_flatten(move || {
///     let local = initialize();
///     C::and_then(fetch(&local), |res| finalize(res))
/// })
/// ```
///
/// - In [`Identity`], there is no suspension concept. The closure `f` is called immediately on the stack,
///   evaluating eagerly.
/// - In [`Async`], `wrap_flatten` acts as the **construction boundary**. The closure `f` is deferred until
///   the returned future is first polled by an executor. This allows variables to be captured safely by the
///   closure without running expensive setup logic at call-site.
///
/// [`Identity`]: crate::Identity
/// [`Async`]: crate::Async
pub trait Wrap: Sized {
    /// Wraps a pure synchronous closure into a `Colored` computation.
    ///
    /// The closure `f` is called to produce a pure value `T`. Under `Async`, this is
    /// deferred until the future is first polled.
    fn wrap<F>(f: F) -> impl Colored<Self, Output = F::Value>
    where
        F: ThunkOnce;

    /// Wraps a closure returning a `Colored` computation, flattening the result.
    ///
    /// # Examples
    ///
    /// ```
    /// # use colorless_core::{Color, Colored, test_util::{fetch_user, fetch_profile, UserProfile}};
    ///
    /// fn func<C: Color>() -> impl Colored<C, Output = UserProfile> {
    ///     // Both chain construction and execution are deferred under Async
    ///     C::wrap_flatten(move || {
    ///         C::and_then(fetch_user(), |user| {
    ///             fetch_profile(user.id)
    ///         })
    ///     })
    /// }
    /// ```
    fn wrap_flatten<F>(f: F) -> impl Colored<Self, Output = F::Value>
    where
        F: ColoredThunkOnce<Self>;
}
