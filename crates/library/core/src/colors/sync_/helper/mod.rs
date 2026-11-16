macro_rules! define_sync {
    (
        $exec:ident,
        <$($g:ident)?> $color:ident $color_value:ident
        $native_t:ty,
        $native:ty,
        $ext:ident $ext_name:ident
    ) => {
        use crate::{
            BorrowScope, BorrowScopeAndThen, BorrowScopeLoop, BorrowScopeMap, BorrowScopeMapMut,
            BorrowUsage, BorrowUsageMap, Colored, Colorize, ColoredLoop, Cps, Erase, Linear,
            LinearBorrow1, LinearBorrowLoop, LocalBorrow, Loop, Ready, Wrap,
            callbacks::{CallOnce, ColoredCallOnce, ColoredThunkOnce, ThunkOnce},
            colored::internal::{ColoredInternal, Present}
        };
        use core::ops::ControlFlow;

        impl$(<$g>)? Ready for $color$(<$g>)? {
            #[inline(always)]
            fn ready<T>(value: T) -> impl Colored<Self, Output = T> {
                $exec!(@init value)
            }
        }

        impl$(<$g>)? Wrap for $color$(<$g>)? {
            #[inline(always)]
            fn wrap<F>(f: F) -> impl Colored<Self, Output = F::Value>
            where
                F: ThunkOnce,
            {
                $exec!(@init f())
            }

            #[inline(always)]
            fn wrap_flatten<F>(f: F) -> impl Colored<Self, Output = F::Value>
            where
                F: ColoredThunkOnce<Self>,
            {
                f()
            }
        }

        impl$(<$g>)? Cps for $color$(<$g>)? {
            #[inline(always)]
            fn map<C, F>(c: C, f: F) -> impl Colored<Self, Output = F::Value>
            where
                C: Colored<Self>,
                F: CallOnce<C::Output>,
            {
                $exec!(@map c.compute_sync().0 => v f(v))
            }

            #[inline(always)]
            fn and_then<C, F>(first: C, next: F) -> impl Colored<Self, Output = F::Value>
            where
                C: Colored<Self>,
                F: ColoredCallOnce<Self, C::Output>,
            {
                $exec!(first.compute_sync().0 => v next(v).compute_sync())
            }
        }

        impl$(<$g>)? Erase for $color$(<$g>)? {
            type Erased<'a, Output> = $color_value<Output $(,$g)?>
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
                computation.compute_sync()
            }
        }

        impl$(<$g>)? LocalBorrow for $color$(<$g>)? {
            #[inline(always)]
            fn borrow<L, S>(mut local: L, s: S) -> impl Colored<Self, Output = S::Resume>
            where
                S: BorrowScope<Self, L>,
            {
                s.resume_mut(&mut local).compute_sync()
            }

            #[inline(always)]
            fn borrow_map<L, S>(mut local: L, s: S) -> impl Colored<Self, Output = S::Output>
            where
                S: BorrowScopeMap<Self, L>,
            {
                let usage = s.resume_mut(&mut local);
                $exec!(@map usage.compute_sync().0 => v v.resume(local))
            }

            #[inline(always)]
            fn borrow_and_then<L, S>(mut local: L, s: S) -> impl Colored<Self, Output = S::Output>
            where
                S: BorrowScopeAndThen<Self, L>,
            {
                let usage = s.resume_mut(&mut local);
                $exec!(usage.compute_sync().0 => v v.resume(local).compute_sync())
            }
        }

        impl$(<$g>)? LinearBorrow1 for $color$(<$g>)? {
            #[inline(always)]
            fn borrow_linear<V, S>(var: &mut V, s: S) -> impl Colored<Self, Output = S::Resume>
            where
                S: BorrowScope<Self, V>,
            {
                s.resume_mut(var).compute_sync()
            }

            #[inline(always)]
            fn borrow_linear_map<'a, V, S>(var: &'a mut V, s: S) -> impl Colored<Self, Output = S::Output>
            where
                S: BorrowScopeMapMut<'a, Self, V>,
            {
                let usage = s.resume_mut(var);
                $exec!(@map usage.compute_sync().0 => v v.resume(var))
            }
        }

        unsafe impl$(<$g>)? Linear for $color$(<$g>)? {}

        impl$(<$g>)? Loop for $color$(<$g>)? {
            #[inline(always)]
            fn loop_<F>(mut f: F) -> impl Colored<Self, Output = F::Break>
            where
                F: ColoredLoop<Self>,
            {
                loop {
                    let control = <_ as ColoredLoop<Self>>::repeat(f);
                    $exec!(@loop control.compute_sync().0 => c f = c)
                }
            }
        }

        impl$(<$g>)? LinearBorrowLoop for $color$(<$g>)? {
            #[inline(always)]
            fn borrow_linear<V, S>(var: &mut V, mut s: S) -> impl Colored<Self, Output = S::Resume>
            where
                S: BorrowScopeLoop<Self, V>,
            {
                loop {
                    let control = s.resume_mut(var);
                    $exec!(@loop control.compute_sync().0 => c s = c)
                }
            }
        }

        impl<T $(, $g)?> Colorize<$color$(<$g>)?> for $native_t {
            type Output = T;

            #[inline(always)]
            fn colorize(self) -> impl Colored<$color$(<$g>)?, Output = Self::Output> {
                $color_value(self)
            }
        }

        unsafe impl<T $(,$g)?> ColoredInternal<$color$(<$g>)?> for $color_value<T $(,$g)?> {
            type OutputInternal = T;

            #[inline(always)]
            fn compute_sync(self) -> <$color$(<$g>)? as Present>::Value<T> {
                self
            }
        }

        impl<T $(,$g)?> Colored<$color$(<$g>)?> for $color_value<T $(,$g)?> {
            type Output = T;
        }

        #[doc = "Extension trait for "]
        #[doc = concat!("[`Colored<", stringify!($color$(<$g>)?), ">`]", stringify!((crate::$color$(<$g>)?)))]
        #[doc = " computations to extract their final value directly."]
        pub trait $ext_name$(<$g>)?: Colored<$color$(<$g>)?> {
            #[doc = "Evaluates the "]
            #[doc = concat!("[`Colored<", stringify!($color$(<$g>)?), ">`]", stringify!((crate::$color$(<$g>)?)))]
            #[doc = " computation and extracts the computed value."]
            #[inline(always)]
            fn into_value(self) -> $native
            where
                Self: Sized,
            {
                self.compute_sync().0
            }

            #[doc = "Evaluates the "]
            #[doc = concat!("[`Colored<", stringify!($color$(<$g>)?), ">`]", stringify!((crate::$color$(<$g>)?)))]
            #[doc = " computation and extracts the computed value."]
            #[inline(always)]
            fn $ext(self) -> $native
            where
                Self: Sized,
            {
                self.compute_sync().0
            }
        }

        impl<T: Colored<$color$(<$g>)?> $(,$g)?> $ext_name$(<$g>)? for T {}

        #[repr(transparent)]
        #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $color_value<T $(,$g)?>(pub(crate) $native_t);

        impl$(<$g>)? Present for $color$(<$g>)? {
            type Value<O> = $color_value<O $(,$g)?>;

            #[cfg(test)]
            fn from<O>(o: O) -> Self::Value<O> {
                $exec!(@init o)
            }
        }

        use $crate::colors::sync_::helper::concrete::SyncOneOf;

        impl_one_of_all! {
            <$($g)?> $color,

            OneOf2,
            T0 => arm_0,
            T1 => arm_1;

            OneOf3,
            T0 => arm_0,
            T1 => arm_1,
            T2 => arm_2;

            OneOf4,
            T0 => arm_0,
            T1 => arm_1,
            T2 => arm_2,
            T3 => arm_3;

            OneOf5,
            T0 => arm_0,
            T1 => arm_1,
            T2 => arm_2,
            T3 => arm_3,
            T4 => arm_4;

            OneOf6,
            T0 => arm_0,
            T1 => arm_1,
            T2 => arm_2,
            T3 => arm_3,
            T4 => arm_4,
            T5 => arm_5;

            OneOf7,
            T0 => arm_0,
            T1 => arm_1,
            T2 => arm_2,
            T3 => arm_3,
            T4 => arm_4,
            T5 => arm_5,
            T6 => arm_6;

            OneOf8,
            T0 => arm_0,
            T1 => arm_1,
            T2 => arm_2,
            T3 => arm_3,
            T4 => arm_4,
            T5 => arm_5,
            T6 => arm_6,
            T7 => arm_7;

            OneOf9,
            T0 => arm_0,
            T1 => arm_1,
            T2 => arm_2,
            T3 => arm_3,
            T4 => arm_4,
            T5 => arm_5,
            T6 => arm_6,
            T7 => arm_7,
            T8 => arm_8;

            OneOf10,
            T0 => arm_0,
            T1 => arm_1,
            T2 => arm_2,
            T3 => arm_3,
            T4 => arm_4,
            T5 => arm_5,
            T6 => arm_6,
            T7 => arm_7,
            T8 => arm_8,
            T9 => arm_9;

            OneOf11,
            T0 => arm_0,
            T1 => arm_1,
            T2 => arm_2,
            T3 => arm_3,
            T4 => arm_4,
            T5 => arm_5,
            T6 => arm_6,
            T7 => arm_7,
            T8 => arm_8,
            T9 => arm_9,
            T10 => arm_10;

            OneOf12,
            T0 => arm_0,
            T1 => arm_1,
            T2 => arm_2,
            T3 => arm_3,
            T4 => arm_4,
            T5 => arm_5,
            T6 => arm_6,
            T7 => arm_7,
            T8 => arm_8,
            T9 => arm_9,
            T10 => arm_10,
            T11 => arm_11
        }
    }
}

macro_rules! impl_one_of_all {
    (
        <> $color:ident,
        $(
            $trait_name:ident,
            $(
                $Tn:ident => $arm_n:ident
            ),+
        );*
    ) => {
        $(
            impl_one_of! {
                <> $color,
                $trait_name,
                $(
                    $Tn => $arm_n
                ),+
            }
        )*
    };
    (
        <$g:ident> $color:ident,
        $(
            $trait_name:ident,
            $(
                $Tn:ident => $arm_n:ident
            ),+
        );*
    ) => {
        $(
            impl_one_of! {
                <$g> $color,
                $trait_name,
                $(
                    $Tn => $arm_n
                ),+
            }
        )*
    };
}

macro_rules! impl_one_of {
    (
        <$($g:ident)?> $color:ident,
        $trait_name:ident,
        $(
            $Tn:ident => $arm_n:ident
        ),+
    ) => {
        impl_one_of! {
            @collected
            <$($g)?> $color,
            $trait_name,
            {concrete: SyncOneOf<Self, ($($Tn,)*), Output> }
            {generics: <$($Tn,)* Output>}
            {bounds: where $(
                $Tn: $crate::Colored<Self, Output = Output>
            ),*;}
            arms: $($Tn => $arm_n)+
        }
    };
    (
        @collected
        <$($g:ident)?> $color:ident,
        $trait_name:ident,
        {concrete: $ty:ty}
        {generics: $($generics:tt)*}
        {bounds: $($bounds:tt)*}
        arms: $($Tn:ident => $arm_n:ident)*
    ) => {
        impl$(<$g>)? $crate::$trait_name for $color$(<$g>)? {
            type Outcome$($generics)* = $ty
            $($bounds)*

            impl_one_of! {
                @arm
                {generics: $($generics)*}
                {all: $($Tn)*}
                {arms: $($Tn => $arm_n)*}
            }
        }
    };
    (
        @arm
        {generics: $($generics:tt)*}
        {all: $($all:ident)*}
        {arms: $T:ident => $arm:ident $($Tn:ident => $arm_n:ident)*}
    ) => {
        #[inline(always)]
        fn $arm$($generics)*(flow: $T) -> Self::Outcome$($generics)*
        where $(
                $all: $crate::Colored<Self, Output = Output>
            ),* {
            SyncOneOf(flow.compute_sync(), ::core::marker::PhantomData)
        }

        impl_one_of! {
            @arm
            {generics: $($generics)*}
            {all: $($all)*}
            {arms: $($Tn => $arm_n)*}
        }
    };
    (
        @arm
        {generics: $($generics:tt)*}
        {all: $($all:tt)*}
        {arms: }
    ) => {}
}

pub(super) mod concrete {
    use crate::Colored;
    use crate::colored::internal::{ColoredInternal, Present};
    use core::marker::PhantomData;

    /// Eager branch result shared by synchronous colors whose native result is
    /// represented by [`Present::Value`]. The selected arm is evaluated before
    /// construction; `Arms` only retains every candidate type for inference.
    #[doc(hidden)]
    pub struct SyncOneOf<C, Arms, Output>(
        pub(crate) <C as Present>::Value<Output>,
        pub(crate) PhantomData<fn() -> (C, Arms)>,
    )
    where
        C: Present;

    impl<C, Arms, Output> Colored<C> for SyncOneOf<C, Arms, Output>
    where
        C: Present,
    {
        type Output = Output;
    }

    unsafe impl<C, Arms, Output> ColoredInternal<C> for SyncOneOf<C, Arms, Output>
    where
        C: Present,
    {
        type OutputInternal = Output;

        #[inline(always)]
        fn compute_sync(self) -> <C as Present>::Value<Output> {
            self.0
        }
    }

    #[cfg(test)]
    mod tests {
        use crate::test_util::fetch;
        use crate::{
            FallibleOption, FallibleOptionExt, Identity, IdentityExt, OneOf2, OneOf4,
            OptionalResult, OptionalResultExt, Wrap,
        };
        use std::hint::black_box;

        #[test]
        fn identity_is_well_typed() {
            let x = black_box(5);
            let v = Identity::wrap_flatten(move || match x {
                0 => <Identity as OneOf4>::arm_0(fetch(1)),
                1 => <Identity as OneOf4>::arm_1(fetch(2)),
                2 => <Identity as OneOf4>::arm_2(fetch(3)),
                _ => <Identity as OneOf4>::arm_3(fetch(4)),
            });

            assert_eq!(v.sync_(), 4);
        }

        #[test]
        fn fallible_option_is_well_typed() {
            let x = black_box(1);
            let v = FallibleOption::<()>::wrap_flatten(move || match x {
                0 => <FallibleOption<()> as OneOf2>::arm_0(fetch(10)),
                _ => <FallibleOption<()> as OneOf2>::arm_1(fetch(20)),
            });

            assert_eq!(v.result_option_(), Ok(Some(20)));
        }

        #[test]
        fn optional_result_is_well_typed() {
            let x = black_box(0);
            let v = OptionalResult::<()>::wrap_flatten(move || match x {
                0 => <OptionalResult<()> as OneOf2>::arm_0(fetch(100)),
                _ => <OptionalResult<()> as OneOf2>::arm_1(fetch(200)),
            });

            assert_eq!(v.option_result_(), Some(Ok(100)));
        }
    }
}
