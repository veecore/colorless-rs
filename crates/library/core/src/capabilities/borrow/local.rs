use crate::{
    BorrowScope, BorrowScopeAndThen, BorrowScopeMap, Colored, Cps, Linear, Ready,
    capabilities::borrow::local::impls::MapScope,
    capabilities::callbacks::{CallOnce, ColoredCallOnce},
};

/// Capability for executing one physical loan stage over an owned local.
///
/// # The problem: address stability across yield points
///
/// When a function creates a local variable and needs to borrow it across
/// a `.resolve` boundary, naive CPS lowering fails:
///
/// ```rust,ignore
/// // Direct-style intent:
/// let mut buffer = String::new();
/// let parsed = parse_header(&mut buffer).resolve;
/// use_buffer(&buffer);
/// ```
///
/// If lowered into combinators the remaining computation must move into
/// a closure — but `buffer` can't be moved while it's borrowed, and even
/// if it could, moving it changes its stack address, invalidating any live
/// reference inside `parse_header`.
///
/// `LocalBorrow` solves this by anchoring the local in a pinned frame that
/// keeps its address stable across the yield. In [`Async`](crate::Async) this
/// produces a `!Unpin` state machine; in synchronous colors it executes as an
/// ordinary stack borrow with no overhead.
///
/// # Nesting rules
///
/// A physical loan stage is not an independently releasable region.
/// Multiple borrows compose as properly nested continuations:
///
/// ```text
/// begin A, begin B, end B, end A   ← directly representable as nested stages
/// begin A, begin B, end A, end B   ← requires segmentation
/// ```
///
/// # Comparison with [`LinearBorrow1`](crate::LinearBorrow1)
///
/// Use `LocalBorrow` when the variable is **created inside the function**.
/// Use [`LinearBorrow1`](crate::LinearBorrow1) when the variable is **passed in as
/// `&mut V`** from the caller — in that case the address is already stable and
/// `LocalBorrow`'s pinning overhead is unnecessary.
pub trait LocalBorrow: Linear + Sized {
    /// Runs one physical stage and returns its result.
    ///
    /// The owned local is dropped when the stage finishes or is canceled; it
    /// is not restored to a usage continuation.
    fn borrow<L, S>(local: L, s: S) -> impl Colored<Self, Output = S::Resume>
    where
        S: BorrowScope<Self, L>;

    /// Runs one physical stage, restores the local, and invokes a synchronous
    /// one-shot usage with ownership of it.
    fn borrow_map<L, S>(local: L, s: S) -> impl Colored<Self, Output = S::Output>
    where
        S: BorrowScopeMap<Self, L>;

    /// Runs one physical stage, restores the local, and invokes a colored
    /// one-shot usage with ownership of it.
    fn borrow_and_then<L, S>(local: L, s: S) -> impl Colored<Self, Output = S::Output>
    where
        S: BorrowScopeAndThen<Self, L>;

    /// Runs a physical stage and passes both the restored owner and the stage
    /// result to a synchronous continuation.
    fn borrow_fold<S, U, L>(local: L, s: S, usage: U) -> impl Colored<Self, Output = U::Value>
    where
        S: BorrowScope<Self, L>,
        U: CallOnce<(L, S::Resume)>,
        Self: Cps + Ready,
    {
        Self::borrow_and_then(
            local,
            MapScope {
                scope: s,
                f: move |resume: S::Resume| {
                    Self::ready(move |local: L| Self::ready(usage((local, resume))))
                },
            },
        )
    }

    /// Runs a physical stage and passes both the restored owner and the stage
    /// result to a colored continuation.
    fn borrow_fold_then<S, U, L>(local: L, s: S, usage: U) -> impl Colored<Self, Output = U::Value>
    where
        S: BorrowScope<Self, L>,
        U: ColoredCallOnce<Self, (L, S::Resume)>,
        Self: Cps + Ready,
    {
        // This is just to convince that borrow_and_then is the more general form
        Self::borrow_and_then(
            local,
            MapScope {
                scope: s,
                f: move |resume: S::Resume| Self::ready(move |local: L| usage((local, resume))),
            },
        )
    }
}

mod impls {
    use crate::capabilities::callbacks::ColoredCallOnce;
    use crate::{BorrowScope, Colored, Cps};

    pub struct MapScope<S, F> {
        pub(crate) scope: S,
        pub(crate) f: F,
    }

    impl<C, Var, S, F> BorrowScope<C, Var> for MapScope<S, F>
    where
        C: Cps,
        S: BorrowScope<C, Var>,
        F: ColoredCallOnce<C, S::Resume>,
    {
        type Resume = F::Value;

        fn resume_mut(self, var: &mut Var) -> impl Colored<C, Output = Self::Resume> {
            let scope = self.scope;
            let f = self.f;
            C::and_then(scope.resume_mut(var), f)
        }
    }
}
