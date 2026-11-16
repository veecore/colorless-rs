use crate::Colored;

/// # Type Erasure and Auto-Trait Preservation
///
/// Monomorphization is the engine of zero-cost abstractions: it inlines code, eliminates dynamic dispatch,
/// and specializes data layouts. But monomorphization has natural boundaries:
///
/// 1. **Open-ended recursion**: A recursive function whose future type grows with every call cannot be
///    represented as a finite static type without boxing (`E0720: opaque type cycle detected`).
/// 2. **Heterogeneous collections**: Storing computations of different concrete origins in a single `Vec`.
/// 3. **Dynamic plugin boundaries**: Hiding internal implementation types across crate boundaries.
///
/// In these situations, type erasure is necessary.
///
/// ---
///
/// ### The Auto-Trait Dilemma
///
/// A common frustration with standard trait objects (e.g `Box<dyn Future>`) is that erasing a concrete type
/// discards all auto traits not explicitly listed in the trait object:
///
/// - `Box<dyn Future<Output = T>>` is `!Send` and `!Sync`, even if the underlying future was thread-safe.
/// - `Box<dyn Future<Output = T> + Send>` works for multi-threaded executors, but rejects single-threaded
///   futures containing `Rc` or local borrows.
///
/// For a library author writing generic code, hardcoding `+ Send` alienates local executors, while omitting
/// it prevents the code from running on multi-threaded runtimes like Tokio.
///
/// However, attempts to solve this issue generically without listing erase_* either rendered the trait not
/// general enough or made the other colors unwieldy.
#[doc(alias = "AnyOf")]
pub trait Erase: Sized {
    /// The erased representation for this color context.
    type Erased<'a, Output>: Colored<Self, Output = Output>
    where
        Output: 'a,
        Self: 'a;

    /// Erases the concrete type `C` of a [`Colored`] computation into [`Self::Erased`].
    ///
    /// # Examples
    /// ```rust
    /// use colorless_core::{Colored, Color, OneOf2, IdentityExt, Erase};
    ///
    /// fn fibonacci<C: Color + Erase>(n: u32) -> impl Colored<C, Output = u32> {
    ///     C::wrap_flatten(move || {
    ///         if n <= 1 {
    ///             <C as OneOf2>::arm_0(C::ready(n))
    ///         } else {
    ///             let step = C::and_then(fibonacci(n - 1), move |a| {
    ///                 C::map(fibonacci(n - 2), move |b| a + b)
    ///             });
    ///             <C as OneOf2>::arm_1(C::erase(step))
    ///         }
    ///     })
    /// }
    ///
    /// assert_eq!(fibonacci(3).sync_(), 2)
    /// ```
    fn erase<'a, C>(computation: C) -> Self::Erased<'a, C::Output>
    where
        C: Colored<Self> + 'a,
        Self: 'a;
}
