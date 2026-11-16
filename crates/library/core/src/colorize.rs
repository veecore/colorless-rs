use crate::Colored;

/// A trait for native host types (like `Option`, `Future`, `Result`) that can
/// be seamlessly converted into their generic [`Colored<C>`] representation.
///
/// # Comparison with [`IntoColored`](crate::IntoColored)
///
/// `Colorize<C>` is for **native host types** that carry a semantic
/// meaning in context `C` — for example, `Option<T>` lifting its
/// presence/absence into the [`Optional`](crate::Optional) short-circuit model, or
/// `impl Future<Output = T>` entering the [`Async`](crate::Async) polling loop.
///
/// [`IntoColored<C>`](crate::IntoColored) is the yield-site protocol used by the proc-macro:
/// it is implemented by builder types (and blanket-implemented by every
/// `Colored<C>`). Prefer `Colorize` when encoding the *meaning* of a host
/// type; prefer `IntoColored` when building a lazy computation structure.
pub trait Colorize<C> {
    /// The target output type produced by the converted computation.
    type Output;

    /// Converts the native value into a [`Colored`] computation.
    fn colorize(self) -> impl Colored<C, Output = Self::Output>;
}
