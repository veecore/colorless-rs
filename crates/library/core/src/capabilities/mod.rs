mod borrow;
mod cps;
mod erase;
mod lifting;
mod loop_;
mod one_of;

pub use borrow::*;
pub use cps::*;
pub use erase::*;
pub use lifting::*;
pub use loop_::*;
pub use one_of::*;

/// The full capability set required by all built-in colors.
///
/// A type satisfies `Color` exactly when it satisfies all primitive capability
/// traits. Generic functions should bound on `Color`:
///
/// ```rust
/// use colorless_core::{Color, Colored};
///
/// fn process<C: Color>(x: i32) -> impl Colored<C, Output = i32> {
///     C::map(C::ready(x), |v| v + 1)
/// }
/// ```
///
/// **Do not bound on individual capability traits** unless you intentionally
/// want to restrict which colors may be used. `Color` is the correct bound in
/// almost every case.
pub trait Color:
    Ready
    + Wrap
    + Cps
    + LocalBorrow
    + Loop
    + OneOf2
    + OneOf3
    + OneOf4
    + OneOf5
    + OneOf6
    + OneOf7
    + OneOf8
    + OneOf9
    + OneOf10
    + OneOf11
    + OneOf12
{
}

impl<T> Color for T where
    T: Ready
        + Wrap
        + Cps
        + LocalBorrow
        + Loop
        + OneOf2
        + OneOf3
        + OneOf4
        + OneOf5
        + OneOf6
        + OneOf7
        + OneOf8
        + OneOf9
        + OneOf10
        + OneOf11
        + OneOf12
{
}

// This module is private and only exported for rt
#[doc(hidden)]
pub mod callbacks {
    //! Callable adapters that keep callback result types associated with the
    //! callbacks that produce them.
    //!
    //! Recursive capability methods must not independently quantify both a
    //! callback `F` and its returned flow `U` through `F: FnOnce(..) -> U`. That
    //! shape causes pathological rustc work in deeply lowered Colorless code.

    use crate::Colored;
    use core::ops::ControlFlow;

    /// Stable access to the result type of a zero-argument `FnOnce`.
    ///
    /// This is separate from [`CallOnce`] because stable Rust cannot express both
    /// callable arities through one direct `FnOnce` supertrait while retaining
    /// contextual closure inference.
    #[doc(hidden)]
    pub trait ThunkOnce: FnOnce() -> Self::Value {
        type Value;
    }

    impl<F, Value> ThunkOnce for F
    where
        F: FnOnce() -> Value,
    {
        type Value = Value;
    }

    /// Stable access to the result type of a one-argument `FnOnce`.
    #[doc(hidden)]
    pub trait CallOnce<A>: FnOnce(A) -> Self::Value {
        type Value;
    }

    impl<F, A, Value> CallOnce<A> for F
    where
        F: FnOnce(A) -> Value,
    {
        type Value = Value;
    }

    /// Stable access to the colored flow returned by a zero-argument callback.
    ///
    /// `Wrap` must store the exact returned flow when flattening a thunk.
    /// Keeping it as `Self::Flow` avoids an independent recursive method
    /// parameter while preserving ordinary `move || ...` syntax.
    #[doc(hidden)]
    pub trait ColoredThunkOnce<C>: FnOnce() -> Self::Flow {
        type Value;
        type Flow: Colored<C, Output = Self::Value>;
    }

    impl<C, F, Value, Flow> ColoredThunkOnce<C> for F
    where
        F: FnOnce() -> Flow,
        Flow: Colored<C, Output = Value>,
    {
        type Value = Value;
        type Flow = Flow;
    }

    /// Stable access to the colored flow returned by a one-argument callback.
    ///
    /// The direct `FnOnce(A)` supertrait is essential: it gives closure parameters
    /// an expected type while rustc is initially inferring an opaque expression.
    #[doc(hidden)]
    pub trait ColoredCallOnce<C, A>: FnOnce(A) -> Self::Flow {
        type Value;
        type Flow: Colored<C, Output = Self::Value>;
    }

    impl<C, F, A, Value, Flow> ColoredCallOnce<C, A> for F
    where
        F: FnOnce(A) -> Flow,
        Flow: Colored<C, Output = Value>,
    {
        type Value = Value;
        type Flow = Flow;
    }

    /// A repeatedly callable loop step whose returned computation is associated
    /// with the callback itself.
    ///
    /// Keeping `Flow` associated prevents recursive loop lowering from making the
    /// compiler independently solve a callback type and its returned flow at every
    /// nesting level.
    pub trait ColoredCallFold<C, State>: FnMut(State) -> Self::Flow {
        /// Value produced when the loop breaks.
        type Break;

        /// Computation for one iteration.
        type Flow: Colored<C, Output = ControlFlow<Self::Break, State>>;
    }

    impl<C, State, Break, Flow, F> ColoredCallFold<C, State> for F
    where
        F: FnMut(State) -> Flow,
        Flow: Colored<C, Output = ControlFlow<Break, State>>,
    {
        type Break = Break;
        type Flow = Flow;
    }

    pub trait ColoredCallFoldMut<C, StateMut, State>:
        FnMut(&mut StateMut, State) -> Self::Flow
    {
        /// Value produced when the loop breaks.
        type Break;

        /// Computation for one iteration.
        type Flow: Colored<C, Output = ControlFlow<Self::Break, State>>;
    }

    impl<C, StateMut, State, Break, Flow, F> ColoredCallFoldMut<C, StateMut, State> for F
    where
        F: FnMut(&mut StateMut, State) -> Flow,
        Flow: Colored<C, Output = ControlFlow<Break, State>>,
    {
        type Break = Break;
        type Flow = Flow;
    }

    pub trait FlatBorrowScope<C, Var> {
        type Resume;
        type Flow: Colored<C, Output = Self::Resume>;
        fn resume_mut_flat(self, var: &mut Var) -> Self::Flow;
    }
}
