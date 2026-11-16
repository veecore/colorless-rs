#[macro_use]
mod adapters;
#[macro_use]
mod wrappers;
#[macro_use]
mod one_of;

pub use one_of::*;

macro_rules! declare_async_all {
    (
        $ext_name:ident,
        <$($g:ident)?> $color:ident
        $native:ty,
        $native_t:ty,
        $ext:ident,
        $native_exec:ident,
        $poll_exec:ident,
    ) => {
#[doc = "Extension trait for "]
#[doc = concat!("[`Colored<", stringify!($color$(<$g>)?), ">`]", stringify!((crate::$color$(<$g>)?)))]
#[doc = " computations to adapt them into native [`Future`]s."]
pub trait $ext_name$(<$g>)?: Colored<$color$(<$g>)?> {
    #[doc = "Converts the "]
    #[doc = concat!("[`Colored<", stringify!($color$(<$g>)?), ">`]", stringify!((crate::$color$(<$g>)?)))]
    #[doc = "computation into an `impl Future`."]
    #[inline(always)]
    fn into_future(self) -> impl Future<Output = $native>
    where
        Self: Sized,
    {
        MakeFuture::new(self)
    }

    #[doc = "Converts the "]
    #[doc = concat!("[`Colored<", stringify!($color$(<$g>)?), ">`]", stringify!((crate::$color$(<$g>)?)))]
    #[doc = "computation into an `impl Future`."]
    #[inline(always)]
    fn $ext(self) -> impl Future<Output = $native>
    where
        Self: Sized,
    {
        MakeFuture::new(self)
    }
}

impl<T: Colored<$color$(<$g>)?> $(,$g)?> $ext_name$(<$g>)? for T {}

impl<T, F $(,$g)?> Colorize<$color$(<$g>)?> for F
where
    F: Future<Output = $native_t>,
{
    type Output = T;

    #[inline(always)]
    fn colorize(self) -> impl Colored<$color$(<$g>)?, Output = Self::Output> {
        MakeColored(self $(, PhantomData::<$g>)?)
    }
}

impl$(<$g>)? Present for $color$(<$g>)? {
    type Value<T> = $native_t;

    #[cfg(test)]
    fn from<O>(o: O) -> Self::Value<O> {
        $native_exec!(@init o)
    }
}

define_async! {
    $native_exec,
    $ext,
    <$($g)?> $color
}

declare_wrappers! {
    $poll_exec,
    <$($g)?> $color
}

declare_adapters! {
    <$($g)?> $color
    $native_t
}

    };
}

#[doc(hidden)]
macro_rules! define_async {
    (
        $exec:ident,
        $ext:ident,
        <$($g:ident)?> $color:ident
    ) => {
        use crate::capabilities::callbacks::{
            CallOnce, ColoredCallFold, ColoredCallOnce, ColoredThunkOnce, ThunkOnce,
        };
        use crate::{
            BorrowScope, BorrowScopeAndThen, BorrowScopeMap, BorrowUsageMap, Colored, Colorize, Cps, Linear,
            LocalBorrow, Ready, Wrap,
        };
        use crate::{
            BorrowScope2, BorrowScopeLoop, BorrowScopeMapMut, BorrowUsage, ColoredLoop, LinearBorrow1,
            LinearBorrow2, LinearBorrowLoop, Loop,
        };
        #[cfg(feature = "alloc")]
        use crate::Erase;
        use crate::colored::internal::{Present, ColoredInternal};
        use core::hint::unreachable_unchecked;
        #[allow(unused_imports)]
        use core::marker::PhantomData;
        #[allow(unused_imports)]
        use core::marker::PhantomPinned;
        use core::mem::ManuallyDrop;
        use core::ops::ControlFlow;
        use core::ops::DerefMut;
        use core::pin::Pin;
        use core::ptr::{drop_in_place, write};
        use core::task::{Context, Poll};

        impl$(<$g>)? Ready for $color$(<$g>)? {
            #[inline(always)]
            fn ready<T>(value: T) -> impl Colored<Self, Output = T> {
                ReadyValue(ManuallyDrop::new(value) $(, PhantomData::<$g>)?)
            }
        }

        impl$(<$g>)? Wrap for $color$(<$g>)? {
            #[inline(always)]
            fn wrap<F>(f: F) -> impl Colored<Self, Output = F::Value>
            where
                F: ThunkOnce,
            {
                AsyncWrap(ManuallyDrop::new(f) $(, PhantomData::<$g>)?)
            }

            #[inline(always)]
            fn wrap_flatten<F>(f: F) -> impl Colored<Self, Output = F::Value>
            where
                F: ColoredThunkOnce<Self>,
            {
                AsyncWrapFlatten::Unresumed {
                    f: ManuallyDrop::new(f),
                }
            }
        }

        impl$(<$g>)? Cps for $color$(<$g>)? {
            #[inline(always)]
            fn map<C, F>(c: C, f: F) -> impl Colored<Self, Output = F::Value>
            where
                C: Colored<Self>,
                F: CallOnce<C::Output>,
            {
                AsyncMap {
                    flow: ManuallyDrop::new(c),
                    map: ManuallyDrop::new(f),
                    $(_marker: PhantomData::<$g>)?
                }
            }

            #[inline(always)]
            fn and_then<C, F>(first: C, next: F) -> impl Colored<Self, Output = F::Value>
            where
                C: Colored<Self>,
                F: ColoredCallOnce<Self, C::Output>,
            {
                // Using AsyncFlatten here reduced our worries but actually defeats our
                // ColoredCallOnce opt and hits the cliff again 🥲
                AsyncAndThen::First {
                    flow: ManuallyDrop::new(first),
                    map: ManuallyDrop::new(next),
                }
            }
        }

        #[cfg(feature = "alloc")]
        impl$(<$g>)? Erase for $color$(<$g>)? {
            type Erased<'a, Output> = AsyncErased<'a, Output $(,$g)?>
            where
                Output: 'a,
                Self: 'a,
            ;

            #[inline(always)]
            fn erase<'a, C>(computation: C) -> Self::Erased<'a, C::Output>
            where
                C: Colored<Self> + 'a,
                Self: 'a
            {
                AsyncErased(::alloc::boxed::Box::new(computation))
            }
        }

        unsafe impl$(<$g>)? Linear for $color$(<$g>)? {}

        impl$(<$g>)? LocalBorrow for $color$(<$g>)? {
            // TODO: SINCE THE COMPILER ACTIVELY USES THIS, WE COULD MANUALLY IMPLEMENT TO AVOID
            //  BLOAT
            #[inline(always)]
            fn borrow<L, S>(mut local: L, s: S) -> impl Colored<Self, Output = S::Resume>
            where
                S: BorrowScope<Self, L>,
            {
                MakeColored(
                    async move {
                        s.resume_mut(&mut local).$ext().await
                    }
                    $(, PhantomData::<$g>)?
                )
            }

            #[inline(always)]
            fn borrow_map<L, S>(mut local: L, s: S) -> impl Colored<Self, Output = S::Output>
            where
                S: BorrowScopeMap<Self, L>,
            {
                MakeColored(
                    async move {
                        let borrow = s.resume_mut(&mut local);
                        let usage = borrow.$ext().await;
                        $exec!(@map usage => v v.resume(local))
                    }
                    $(, PhantomData::<$g>)?
                )
            }

            #[inline(always)]
            fn borrow_and_then<L, S>(mut local: L, s: S) -> impl Colored<Self, Output = S::Output>
            where
                S: BorrowScopeAndThen<Self, L>,
            {
                MakeColored(
                    async move {
                        let borrow = s.resume_mut(&mut local);
                        let usage = borrow.$ext().await;
                        $exec!(usage => v v.resume(local).$ext().await)
                    }
                    $(, PhantomData::<$g>)?
                )
            }

            fn borrow_fold<S, U, L>(mut local: L, s: S, usage: U) -> impl Colored<Self, Output = U::Value>
            where
                S: BorrowScope<Self, L>,
                U: CallOnce<(L, S::Resume)>,
                Self: Cps + Ready,
            {
                MakeColored(
                    async move {
                        let borrow = s.resume_mut(&mut local);
                        let borrow_usage = borrow.$ext().await;
                        $exec!(@map borrow_usage => v usage((local, v)))
                    }
                    $(, PhantomData::<$g>)?
                )
            }

            fn borrow_fold_then<S, U, L>(
                mut local: L,
                s: S,
                usage: U,
            ) -> impl Colored<Self, Output = U::Value>
            where
                S: BorrowScope<Self, L>,
                U: ColoredCallOnce<Self, (L, S::Resume)>,
                Self: Cps + Ready,
            {
                MakeColored(
                    async move {
                        let borrow = s.resume_mut(&mut local);
                        let borrow_usage = borrow.$ext().await;
                        $exec!(borrow_usage => v usage((local, v)).$ext().await)
                    }
                    $(, PhantomData::<$g>)?
                )
            }
        }

        impl$(<$g>)? LinearBorrow1 for $color$(<$g>)? {
            #[inline(always)]
            fn borrow_linear<V, S>(var: &mut V, s: S) -> impl Colored<Self, Output = S::Resume>
            where
                S: BorrowScope<Self, V>,
            {
                MakeColored(
                    async move {
                        let borrow = s.resume_mut(var);
                        borrow.$ext().await
                    }
                    $(, PhantomData::<$g>)?
                )
            }

            #[inline(always)]
            fn borrow_linear_map<'a, V, S>(var: &'a mut V, s: S) -> impl Colored<Self, Output = S::Output>
            where
                S: BorrowScopeMapMut<'a, Self, V>,
            {
                MakeColored(
                    async move {
                        let borrow = s.resume_mut(var);
                        let usage = borrow.$ext().await;
                        $exec!(@map usage => v v.resume(var))
                    }
                    $(, PhantomData::<$g>)?
                )
            }
        }

        impl$(<$g>)? LinearBorrow2 for $color$(<$g>)? {
            #[inline(always)]
            fn borrow_linear<V, S>(var: &mut V, s: S) -> impl Colored<Self, Output = S::Resume>
            where
                S: BorrowScope2<Self, V>,
            {
                MakeColored(
                    async move {
                        let borrow = s.resume_mut(var).$ext().await;
                        $exec!(borrow => v v.resume_mut(var).$ext().await)
                    }
                    $(, PhantomData::<$g>)?
                )
            }
        }

        impl$(<$g>)? Loop for $color$(<$g>)? {
            #[inline(always)]
            fn loop_<F>(mut f: F) -> impl Colored<Self, Output = F::Break>
            where
                F: ColoredLoop<Self>,
            {
                MakeColored(
                    async move {
                        loop {
                            let c = <_ as ColoredLoop<Self>>::repeat(f);
                            let control = c.$ext().await;
                            $exec!(@loop control => x f = x)
                        }
                    }
                    $(, PhantomData::<$g>)?
                )
            }

            #[inline(always)]
            fn fold<S, F>(state: S, step: F) -> impl Colored<Self, Output = F::Break>
            where
                F: ColoredCallFold<Self, S>,
                Self: Cps,
            {
                AsyncFold {
                    f: ManuallyDrop::new(step),
                    phase: LoopFoldPhase::Unresumed(ManuallyDrop::new(state)),
                }
            }
        }

        impl$(<$g>)? LinearBorrowLoop for $color$(<$g>)? {
            #[inline(always)]
            fn borrow_linear<V, S>(var: &mut V, mut s: S) -> impl Colored<Self, Output = S::Resume>
            where
                S: BorrowScopeLoop<Self, V>,
            {
                MakeColored(
                    async move {
                        loop {
                            let control = s.resume_mut(var).$ext().await;
                            $exec!(@loop control => x s = x)
                        }
                    }
                    $(, PhantomData::<$g>)?
                )
            }
        }
    };
}
