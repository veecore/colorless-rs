use crate::{BorrowScope, BorrowScopeMapMut, Colored};
pub use borrow_scopes::*;

/// Single-stage borrow runner for caller-owned `&mut V` references.
///
/// # The problem: call-site evaluation mismatch
///
/// When a function receives `var: &mut V` from the caller, lowering two
/// sequential uses of it into CPS fails at the call site:
///
/// ```rust,ignore
/// fn process<C: Color>(reader: &mut Reader) -> impl Colored<C, Output = ()> {
///     // Direct-style intent — both .resolve uses are sequential:
///     let first  = reader.read_chunk().resolve;
///     let second = reader.read_chunk().resolve;
/// }
///
/// // Naive CPS lowering — rustc rejects with E0505:
/// C::and_then(reader.read_chunk(), move |_| {
///     reader.read_chunk()   // ← captured mutably while already borrowed above
/// })
/// ```
///
/// Rust evaluates both arguments at the call site simultaneously. It sees
/// `reader.read_chunk()` borrowing `reader` mutably while the closure
/// also captures `reader` mutably, and cannot prove the closure only runs
/// after the first borrow ends.
///
/// # Why not [`LocalBorrow`](crate::LocalBorrow)?
///
/// The caller's `&mut V` is already address-stable — the caller owns the
/// storage and guarantees it lives for the duration of the call.
/// Routing through `LocalBorrow` would add an unnecessary extra level of
/// indirection (`&mut &mut Reader`) without solving anything new.
///
/// # Solution: inverted-control stage runner
///
/// `LinearBorrow1::borrow_linear` holds `&mut V` itself and passes it into
/// a [`BorrowScope`] for each sequential stage. The runtime guarantees that
/// stage N's borrow ends before stage N+1 begins, satisfying the borrow
/// checker without unsafe code.
///
/// For two sequential stages on the same reference use [`LinearBorrow2`].
/// For loop-driven repeated borrows use [`LinearBorrowLoop`].
///
/// For references created inside the function body use [`LocalBorrow`](crate::LocalBorrow).
pub trait LinearBorrow1: Sized {
    /// Runs `s` with mutable access to `var`, returning its result.
    ///
    /// Use this when a function receives `var: &mut V` from the caller and
    /// needs to pass it to a colored computation without hitting the
    /// call-site double-borrow restriction.
    fn borrow_linear<V, S>(var: &mut V, s: S) -> impl Colored<Self, Output = S::Resume>
    where
        S: BorrowScope<Self, V>;

    /// Runs `s` with mutable access to `var`, then maps the result through
    /// the continuation stored in the [`BorrowScopeMapMut`] resume type.
    ///
    /// The continuation receives `&'a mut V` after the scope completes,
    /// allowing further mutation of `var` without an extra borrow stage.
    fn borrow_linear_map<'a, V, S>(var: &'a mut V, s: S) -> impl Colored<Self, Output = S::Output>
    where
        S: BorrowScopeMapMut<'a, Self, V>;
}

/// Two-stage borrow runner for the same `&mut V` reference.
///
/// Use when a function needs to borrow `var: &mut V` across **two**
/// sequential yield points. The first stage calls `resume_mut` on the
/// outer scope; when it completes, the intermediate [`BorrowScope`] is
/// handed `var` again for the second stage.
///
/// For a single borrow stage use [`LinearBorrow1`]. For loop-driven
/// repeated borrows use [`LinearBorrowLoop`].
pub trait LinearBorrow2: Sized {
    /// Drives a two-stage borrow of `var` through scope `s`.
    fn borrow_linear<V, S>(var: &mut V, s: S) -> impl Colored<Self, Output = S::Resume>
    where
        S: BorrowScope2<Self, V>;
}

/// Loop-driven borrow runner for the same `&mut V` reference.
///
/// Use when a colored loop needs repeated mutable access to an external
/// reference across each iteration's yield point. Each call to
/// `BorrowScopeLoop::resume_mut` receives `var` and returns either
/// `ControlFlow::Continue` (loop again) or `ControlFlow::Break` (done).
///
/// For non-loop borrows see [`LinearBorrow1`] and [`LinearBorrow2`].
pub trait LinearBorrowLoop: Sized {
    /// Drives a repeated borrow of `var` through loop scope `s` until it
    /// returns `ControlFlow::Break`.
    fn borrow_linear<V, S>(var: &mut V, s: S) -> impl Colored<Self, Output = S::Resume>
    where
        S: BorrowScopeLoop<Self, V>;
}

mod borrow_scopes {
    use crate::{BorrowScope, Colored};
    use core::ops::ControlFlow;

    /// A two-stage borrow scope: the first stage yields a [`BorrowScope`]
    /// for the second stage.
    ///
    /// `resume_mut` is called with `var` to produce a computation that
    /// resolves to `Self::Borrow1`. Once that computation completes,
    /// `Borrow1::resume_mut` is called with `var` again for the second
    /// borrow stage.
    pub trait BorrowScope2<C, Var> {
        /// The final output produced after both borrow stages complete.
        type Resume;

        /// The intermediate scope that handles the second borrow stage.
        type Borrow1: BorrowScope<C, Var, Resume = Self::Resume>;

        /// Begins the first borrow stage, returning the computation that
        /// yields the second-stage scope.
        fn resume_mut(self, var: &mut Var) -> impl Colored<C, Output = Self::Borrow1>;
    }

    /// A loop-iteration borrow scope: each call receives `var` mutably and
    /// signals whether to continue or break.
    ///
    /// Returning `ControlFlow::Continue(self)` starts the next iteration.
    /// Returning `ControlFlow::Break(value)` exits the loop with `value`
    /// as the result of [`LinearBorrowLoop::borrow_linear`](crate::LinearBorrowLoop::borrow_linear).
    pub trait BorrowScopeLoop<C, Var> {
        /// The value produced when the loop exits via [`ControlFlow::Break`].
        type Resume;

        /// Runs one loop iteration with mutable access to `var`.
        fn resume_mut(
            self,
            var: &mut Var,
        ) -> impl Colored<C, Output = ControlFlow<Self::Resume, Self>>
        where
            Self: Sized;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AsyncExt, BorrowUsageMap, Color, test_util::block_on_test};
    use std::ops::DerefMut;
    use std::prelude::v1::String;

    #[test]
    fn linear_borrow() {
        // Here's the code we're trying to write
        /*
            // x is already pinned with the parameter
            // so local borrow would be redundant
            let o = first_reference_twice(x).resolve;
            println!("{:?}", o); We use o
            last_reference_single(x).resolve;
        */
        // with the traditional Cps, this fails because both the first's colored output
        // and the closure reference x mutably at once... rust can't prove that the args
        // to these Cps functions are ordered... to rust, it could've well-read and_then
        // (closure, color)..

        // twice because even its output references it
        fn first_reference_twice<C: Color>(x: &mut String) -> impl Colored<C, Output = &mut str> {
            C::wrap(move || x.deref_mut())
        }

        // it also works if this also borrows output
        fn last_reference_single<C: Color>(_x: &mut String) -> impl Colored<C, Output = ()> {
            C::ready(())
        }

        struct LiBImpl;

        impl<C: Color> BorrowScope2<C, String> for LiBImpl {
            type Resume = ();
            type Borrow1 = LiBImpl2;

            fn resume_mut(self, var: &mut String) -> impl Colored<C, Output = Self::Borrow1> {
                let o = first_reference_twice(var);
                // Apparently, we still use the Cps functions but just for returning a higher computation
                C::map(o, move |o| {
                    o.make_ascii_lowercase();

                    LiBImpl2
                })
            }
        }

        struct LiBImpl2;

        impl<C: Color> BorrowScope<C, String> for LiBImpl2 {
            type Resume = ();

            fn resume_mut(self, var: &mut String) -> impl Colored<C, Output = Self::Resume> {
                // We could reinitiate borrow here to simulate more stages
                last_reference_single(var)
            }
        }

        fn func<C: Color + LinearBorrow2>(x: &mut String) -> impl Colored<C, Output = ()> {
            C::borrow_linear(x, LiBImpl)
        }

        let mut s = String::from("HELLO WORLD");
        block_on_test(func(&mut s).async_());
        assert_eq!(s, "hello world");
    }

    #[test]
    fn linear_borrow_map() {
        /*
            let o = reference_x(x).resolve;
            o.make_ascii_lowercase();
            x
        */

        fn reference_x<C: Color>(x: &mut String) -> impl Colored<C, Output = &mut str> {
            C::wrap(|| x.deref_mut())
        }

        // Try Cps... as expected, it fails
        /*
            fn func<C: Color>(x: &mut String) -> impl Colored<C, Output = &mut String> {
                C::map(reference_x(x), move |o| {
                    o.make_ascii_lowercase();
                    x
                })
            }
        */

        struct LiBImpl;

        impl<C: Color> BorrowScope<C, String> for LiBImpl {
            type Resume = LiMap;

            fn resume_mut(self, var: &mut String) -> impl Colored<C, Output = Self::Resume> {
                let o = reference_x(var);
                // Apparently, we still use the Cps functions but just for returning a higher computation
                C::map(o, move |o| {
                    o.make_ascii_lowercase();

                    LiMap
                })
            }
        }

        struct LiMap;

        impl<'a> BorrowUsageMap<&'a mut String> for LiMap {
            type Output = &'a mut String;

            fn resume(self, local: &'a mut String) -> Self::Output {
                local
            }
        }

        fn func<C: Color + LinearBorrow1>(x: &mut String) -> impl Colored<C, Output = &mut String> {
            C::borrow_linear_map(x, LiBImpl)
        }

        let mut s = String::from("HELLO WORLD");
        block_on_test(func(&mut s).async_());
        assert_eq!(s, "hello world");
    }
}
