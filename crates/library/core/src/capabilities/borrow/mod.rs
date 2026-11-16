mod linear;
mod local;

pub use linear::*;
pub use local::*;

pub use aliases::*;
pub use borrow_scope::*;
pub use borrow_usage::*;

mod borrow_scope {
    use crate::Colored;

    /// A state machine trait representing a stage where local state `Var` is mutably borrowed
    /// during an intermediate yield.
    pub trait BorrowScope<C, Var> {
        /// The value produced when the borrow scope completes.
        type Resume;

        /// Resume the scope that requires var mutably
        fn resume_mut(self, var: &mut Var) -> impl Colored<C, Output = Self::Resume>;
    }
}

// finalizers
mod borrow_usage {
    use crate::Colored;

    /// A one-shot continuation invoked after one physical loan stage has ended.
    ///
    /// Unlike `FnOnce`, this trait can be implemented by heterogeneous enums. That
    /// lets separate ending paths retain distinct continuation types while
    /// presenting one statically known usage type to [`LocalBorrow`](crate::LocalBorrow).
    pub trait BorrowUsageMap<Local> {
        /// The value produced by this continuation.
        type Output;

        /// Resumes execution with ownership of the restored local.
        fn resume(self, local: Local) -> Self::Output;
    }

    /// A one-shot continuation that resumes after ownership of `Local` has been
    /// restored.
    ///
    /// Unlike `FnOnce`, this trait can be implemented by heterogeneous enums. That
    /// lets separate paths retain their distinct closure types while
    /// presenting one statically known continuation to [`LocalBorrow`](crate::LocalBorrow).
    pub trait BorrowUsage<C, Local> {
        /// Value produced after the restored-local continuation completes.
        type Output;

        /// Resume execution with ownership of the local value.
        fn resume(self, local: Local) -> impl Colored<C, Output = Self::Output>;
    }
}

mod aliases {
    use crate::{BorrowScope, BorrowUsage, BorrowUsageMap};

    /// Alias combining [`BorrowScope`] with a [`BorrowUsageMap`] resume type.
    ///
    /// A scope that implements this trait can be driven by
    /// [`LocalBorrow::borrow_map`](crate::LocalBorrow::borrow_map): after the scope completes, the resume
    /// value is passed synchronously to `Map::resume` to produce the final
    /// output.
    pub trait BorrowScopeMap<C, L>: BorrowScope<C, L, Resume = Self::Map> {
        /// The final value produced after the resume continuation runs.
        type Output;

        /// The synchronous continuation that receives ownership of `L` and
        /// produces [`Self::Output`].
        type Map: BorrowUsageMap<L, Output = Self::Output>;
    }

    /// Alias combining [`BorrowScope`] with a [`BorrowUsage`] resume type.
    ///
    /// Like [`BorrowScopeMap`] but the resume continuation itself returns a
    /// `Colored<C>` computation, allowing further stages after the borrow ends.
    pub trait BorrowScopeAndThen<C, L>: BorrowScope<C, L, Resume = Self::Map> {
        /// The final value produced after the colored resume continuation
        /// resolves.
        type Output;

        /// The colored continuation that receives ownership of `L` and
        /// returns a computation yielding [`Self::Output`].
        type Map: BorrowUsage<C, L, Output = Self::Output>;
    }

    /// Alias combining [`BorrowScope`] with a [`BorrowUsageMap`] that
    /// receives `&'a mut L` rather than owned `L`.
    ///
    /// Used by [`LinearBorrow1::borrow_linear_map`](crate::LinearBorrow1::borrow_linear_map)
    /// when the continuation needs to keep mutating the caller's variable after the scope ends.
    pub trait BorrowScopeMapMut<'a, C, L: 'a>: BorrowScope<C, L, Resume = Self::Map> {
        /// The final value produced after the resume continuation runs.
        type Output;

        /// The synchronous continuation that receives `&'a mut L` and
        /// produces [`Self::Output`].
        type Map: BorrowUsageMap<&'a mut L, Output = Self::Output>;
    }
}

mod impls {
    use crate::{
        BorrowScope, BorrowScopeAndThen, BorrowScopeMap, BorrowScopeMapMut, BorrowUsage,
        BorrowUsageMap, Colored,
    };

    impl<T, C, L> BorrowScopeMap<C, L> for T
    where
        T: BorrowScope<C, L>,
        T::Resume: BorrowUsageMap<L>,
    {
        type Output = <T::Resume as BorrowUsageMap<L>>::Output;

        type Map = T::Resume;
    }

    impl<T, C, L> BorrowScopeAndThen<C, L> for T
    where
        T: BorrowScope<C, L>,
        T::Resume: BorrowUsage<C, L>,
    {
        type Output = <T::Resume as BorrowUsage<C, L>>::Output;

        type Map = T::Resume;
    }

    impl<'a, T, C, L: 'a> BorrowScopeMapMut<'a, C, L> for T
    where
        T: BorrowScope<C, L>,
        T::Resume: BorrowUsageMap<&'a mut L>,
    {
        type Output = <T::Resume as BorrowUsageMap<&'a mut L>>::Output;

        type Map = T::Resume;
    }

    impl<Local, Output, F> BorrowUsageMap<Local> for F
    where
        F: FnOnce(Local) -> Output,
    {
        type Output = Output;

        #[inline(always)]
        fn resume(self, local: Local) -> Self::Output {
            self(local)
        }
    }

    impl<C, Local, Output, Flow, F> BorrowUsage<C, Local> for F
    where
        F: FnOnce(Local) -> Flow,
        Flow: Colored<C, Output = Output>,
    {
        type Output = Output;

        #[inline(always)]
        fn resume(self, local: Local) -> impl Colored<C, Output = Self::Output> {
            self(local)
        }
    }

    // it's kinda pointless since fn isn't general over borrow
    impl<W, F, C, Var> BorrowScope<C, Var> for F
    where
        W: Colored<C>,
        F: FnOnce(&mut Var) -> W,
    {
        type Resume = W::Output;

        #[inline(always)]
        fn resume_mut(self, var: &mut Var) -> impl Colored<C, Output = Self::Resume> {
            self(var)
        }
    }
}
