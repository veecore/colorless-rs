macro_rules! impl_one_of_trait {
    (
        <$($g:ident)?> $color:ident,
        $trait_name:ident, $concrete:ident;
        $(
            $Tn:ident => $arm_n:ident,
        )+
    ) => {
        impl_one_of_trait! {
            @collected
            <$($g)?> $color,
            $trait_name,
            {concrete: $concrete<$($Tn,)*>, name: $concrete}
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
        {concrete: $ty:ty, name: $concrete_name:ident}
        {generics: $($generics:tt)*}
        {bounds: $($bounds:tt)*}
        arms: $($Tn:ident => $arm_n:ident)*
    ) => {
        impl$(<$g>)? $crate::$trait_name for $color$(<$g>)? {
            type Outcome$($generics)* = $ty
            $($bounds)*

            impl_one_of_trait! {
                @arm
                {concrete: $concrete_name}
                {generics: $($generics)*}
                {all: $($Tn)*}
                {arms: $($Tn => $arm_n)*}
            }
        }
    };
    (
        @arm
        {concrete: $ty:ident}
        {generics: $($generics:tt)*}
        {all: $($all:ident)*}
        {arms: $T:ident => $arm:ident $($Tn:ident => $arm_n:ident)*}
    ) => {
        #[inline(always)]
        fn $arm$($generics)*(flow: $T) -> Self::Outcome$($generics)*
        where $(
                $all: $crate::Colored<Self, Output = Output>
            ),* {

            $ty::$T(flow)
        }

        impl_one_of_trait! {
            @arm
            {concrete: $ty}
            {generics: $($generics)*}
            {all: $($all)*}
            {arms: $($Tn => $arm_n)*}
        }
    };
    (
        @arm
        {concrete: $ty:ident}
        {generics: $($generics:tt)*}
        {all: $($all:tt)*}
        {arms: }
    ) => {}
}

macro_rules! impl_one_of_concrete {
    (
        <$($g:ident)?> $color:ident($color_ty:ty)
        $trait_name:ident, $concrete:ident;
        $(
            $Tn:ident => $arm_n:ident,
        )+
    ) => {
        unsafe impl<$($Tn,)* Output $(,$g)?> $crate::internal::ColoredInternal<$color_ty> for $concrete<$($Tn,)*>
        where
            $($Tn: $crate::Colored<$color_ty, Output = Output>,)*
        {
            type OutputInternal = Output;

            #[inline(always)]
            unsafe fn poll_colored(
                self: Pin<&mut Self>,
                cx: &mut Context<'_>,
            ) -> Poll<<$color$(<$g>)? as Present>::Value<Self::OutputInternal>> {
                match unsafe { self.get_unchecked_mut() } {
                    $(
                        $concrete::$Tn(f) => unsafe {
                            Pin::new_unchecked(f).poll_colored(cx)
                        },
                    )*
                }
            }

            #[inline(always)]
            unsafe fn finalize_ready(self: Pin<&mut Self>) {
                match unsafe { self.get_unchecked_mut() } {
                    $(
                        $concrete::$Tn(f) => unsafe {
                            Pin::new_unchecked(f).finalize_ready()
                        },
                    )*
                }
            }
        }

        impl<$($Tn,)* Output $(,$g)?> crate::Colored<$color_ty> for $concrete<$($Tn,)*> where
            $($Tn: $crate::Colored<$color_ty, Output = Output>,)*
        {
            type Output = Output;
        }
    }
}

macro_rules! declare_one_of_concrete {
    (
        $concrete:ident,
        $($Tn:ident),*
    ) => {
        /// Unified unboxed enum outcome wrapping one of N active async branches.
        pub enum $concrete<$($Tn,)*> {
            $(
                #[allow(missing_docs)]
                $Tn($Tn),
            )*
        }
    };
}

macro_rules! impl_one_of {
    (
        <$($g:ident)?> $color:ident,
        $trait_name:ident, $concrete:ident;
        $(
            $Tn:ident => $arm_n:ident,
        )+
    ) => {
        impl_one_of_concrete! {
            <$($g)?> $color($color $(<$g>)?)
            $trait_name, $concrete;
            $(
                $Tn => $arm_n,
            )*
        }

        impl_one_of_trait! {
            <$($g)?> $color,
            $trait_name, $concrete;
            $(
                $Tn => $arm_n,
            )+
        }
    };
}

macro_rules! impl_one_of_all {
    (
        <> $color:ident,
        $(
            $trait_name:ident, $concrete:ident;
            $(
                $Tn:ident => $arm_n:ident
            ),+
        );*
    ) => {
        $(
            impl_one_of! {
                <> $color,
                $trait_name, $concrete;
                $(
                    $Tn => $arm_n,
                )+
            }
        )*
    };
    (
        <$g:ident> $color:ident,
        $(
            $trait_name:ident, $concrete:ident;
            $(
                $Tn:ident => $arm_n:ident
            ),+
        );*
    ) => {
        $(
            impl_one_of! {
                <$g> $color,
                $trait_name, $concrete;
                $(
                    $Tn => $arm_n,
                )+
            }
        )*
    };
}

declare_one_of_concrete! {
    OneOf2Outcome,
    T0,
    T1
}

declare_one_of_concrete! {
    OneOf3Outcome,
    T0,
    T1,
    T2
}

declare_one_of_concrete! {
    OneOf4Outcome,
    T0,
    T1,
    T2,
    T3
}

declare_one_of_concrete! {
    OneOf5Outcome,
    T0,
    T1,
    T2,
    T3,
    T4
}

declare_one_of_concrete! {
    OneOf6Outcome,
    T0,
    T1,
    T2,
    T3,
    T4,
    T5
}

declare_one_of_concrete! {
    OneOf7Outcome,
    T0,
    T1,
    T2,
    T3,
    T4,
    T5,
    T6
}

declare_one_of_concrete! {
    OneOf8Outcome,
    T0,
    T1,
    T2,
    T3,
    T4,
    T5,
    T6,
    T7
}

declare_one_of_concrete! {
    OneOf9Outcome,
    T0,
    T1,
    T2,
    T3,
    T4,
    T5,
    T6,
    T7,
    T8
}

declare_one_of_concrete! {
    OneOf10Outcome,
    T0,
    T1,
    T2,
    T3,
    T4,
    T5,
    T6,
    T7,
    T8,
    T9
}

declare_one_of_concrete! {
    OneOf11Outcome,
    T0,
    T1,
    T2,
    T3,
    T4,
    T5,
    T6,
    T7,
    T8,
    T9,
    T10
}

declare_one_of_concrete! {
    OneOf12Outcome,
    T0,
    T1,
    T2,
    T3,
    T4,
    T5,
    T6,
    T7,
    T8,
    T9,
    T10,
    T11
}
