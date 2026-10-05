//! Concrete signal level aliases (`L1Signal` through `L9Signal`) and capability implementations.

use crate::level0::{L0Signal, Never};
use crate::level_n::LnSignal;
use crate::traits::*;

macro_rules! impl_local_traits {
    (
        $signal:ident, ($($G:ident),*),
        $can_break_local:ident, $break_local:ident, $LB:ident,
        $can_continue_parent:ident, $continue_parent:ident, $CP:ident,
    ) => {
        impl<$($G),*> $can_break_local for $signal<$($G),*> {
            type $LB = $LB;

            #[inline(always)]
            fn $break_local(b: Self::$LB) -> Self {
                Self::$break_local(b)
            }
        }

        impl<$($G),*> $can_continue_parent for $signal<$($G),*> {
            type $CP = $CP;

            #[inline(always)]
            fn $continue_parent(c: Self::$CP) -> Self {
                Self::$continue_parent(c)
            }
        }

        impl<$($G),*> CanBreak0 for $signal<$($G),*> {
            type B0 = B0;

            #[inline(always)]
            fn break_0(b: Self::B0) -> Self {
                Self::break_0(b)
            }
        }

        impl<$($G),*> CanReturn for $signal<$($G),*> {
            type R = R;

            #[inline(always)]
            fn return_(r: Self::R) -> Self {
                Self::return_(r)
            }
        }
    };
}

macro_rules! impl_outer_trait {
    (
        $signal:ident, ($($G:ident),*),
        $can_break_outer:ident, $break_outer:ident, $B:ident,
        $can_continue_outer:ident, $continue_outer:ident, $C:ident,
    ) => {
        impl<$($G),*> $can_break_outer for $signal<$($G),*> {
            type $B = $B;

            #[inline(always)]
            fn $break_outer(b: Self::$B) -> Self {
                Self::$break_outer(b)
            }
        }

        impl<$($G),*> $can_continue_outer for $signal<$($G),*> {
            type $C = $C;

            #[inline(always)]
            fn $continue_outer(c: Self::$C) -> Self {
                Self::$continue_outer(c)
            }
        }
    };
}

macro_rules! impl_loop_signal_traits {
    (
        $signal:ident;
        $generics:tt;
        $can_break_local:ident, $break_local:ident, $LB:ident;
        $can_continue_parent:ident, $continue_parent:ident, $CP:ident;
        $(
            ($break_outer:ident($B:ident): $can_break_outer:ident, $continue_outer:ident($C:ident): $can_continue_outer:ident)
        )*
    ) => {
        impl_local_traits! {
            $signal, $generics,
            $can_break_local, $break_local, $LB,
            $can_continue_parent, $continue_parent, $CP,
        }

        $(
            impl_outer_trait! {
                $signal, $generics,
                $can_break_outer, $break_outer, $B,
                $can_continue_outer, $continue_outer, $C,
            }
        )*
    };
}

macro_rules! define_loop_signal {
    (
        $depth:literal

        $no_outer:ident;
        $no_outer_no_break_local:ident;

        $no_outer_no_continue_parent:ident;
        $no_outer_no_break_local_no_continue_parent:ident;

        $signal:ident($parent_signal:ident)
            ($break_local:ident($LB:ident): $can_break_local:ident, $continue_parent:ident($CP:ident): $can_continue_parent:ident)
            $(
                ($break_outer:ident($B:ident): $can_break_outer:ident, $continue_outer:ident($C:ident): $can_continue_outer:ident)
            )*
    ) => {
        pub type $signal<$LB, $CP, $($B = Never, $C = Never,)* B0 = Never, R = Never> = LnSignal<$LB, $CP, $parent_signal<$($B, $C,)* B0, R>>;
        /// A signal type with no signal that propagates to an outer scope.
        pub type $no_outer<$LB, $CP> = $signal<$LB, $CP>;
        pub type $no_outer_no_continue_parent<$LB> = $signal<$LB, Never>;
        /// A signal type that cannot propagate outward and cannot break from the current loop.
        pub type $no_outer_no_break_local<$CP> = $signal<Never, $CP>;
        pub type $no_outer_no_break_local_no_continue_parent = $signal<Never, Never>;

        impl<$LB, $CP, $($B, $C,)* B0, R> $signal<$LB, $CP, $($B, $C,)* B0, R> {
            /// Creates a signal that breaks from the current loop.
            #[inline(always)]
            pub const fn $break_local(b: $LB) -> Self {
                Self::BreakLocal(b)
            }

            /// Creates a signal that continues the immediately enclosing loop.
            #[inline(always)]
            pub const fn $continue_parent(c: $CP) -> Self {
                Self::ContinueParent(c)
            }

            $(
                /// Creates a signal that breaks from an enclosing loop.
                #[inline(always)]
                pub const fn $break_outer(b: $B) -> Self {
                    Self::Outer($parent_signal::$break_outer(b))
                }

                /// Creates a signal that continues an enclosing loop.
                #[inline(always)]
                pub const fn $continue_outer(c: $C) -> Self {
                    Self::Outer($parent_signal::$continue_outer(c))
                }
            )*

            /// Creates a signal that breaks from the outermost loop.
            #[inline(always)]
            pub const fn break_0(b: B0) -> Self {
                Self::Outer($parent_signal::break_0(b))
            }

            /// Creates a signal that returns from the enclosing function.
            #[inline(always)]
            pub const fn return_(r: R) -> Self {
                Self::Outer($parent_signal::return_(r))
            }
        }

        impl_loop_signal_traits! {
            $signal;
            ($LB, $CP, $($B, $C,)* B0, R);
            $can_break_local, $break_local, $LB;
            $can_continue_parent, $continue_parent, $CP;
            $(
                ($break_outer($B): $can_break_outer, $continue_outer($C): $can_continue_outer)
            )*
        }
    };
}

define_loop_signal! {
    1
    L1SignalNoOuter;
    L1SignalNoOuterNoBreakLocal;
    L1SignalNoOuterNoContinueParent;
    L1SignalNoOuterNoBreakLocalNoContinueParent;

    L1Signal(L0Signal)
        (break_1(B1): CanBreak1, continue_0(C0): CanContinue0)
}

define_loop_signal! {
    2
    L2SignalNoOuter;
    L2SignalNoOuterNoBreakLocal;
    L2SignalNoOuterNoContinueParent;
    L2SignalNoOuterNoBreakLocalNoContinueParent;

    L2Signal(L1Signal)
        (break_2(B2): CanBreak2, continue_1(C1): CanContinue1)
        (break_1(B1): CanBreak1, continue_0(C0): CanContinue0)
}

define_loop_signal! {
    3
    L3SignalNoOuter;
    L3SignalNoOuterNoBreakLocal;
    L3SignalNoOuterNoContinueParent;
    L3SignalNoOuterNoBreakLocalNoContinueParent;

    L3Signal(L2Signal)
        (break_3(B3): CanBreak3, continue_2(C2): CanContinue2)
        (break_2(B2): CanBreak2, continue_1(C1): CanContinue1)
        (break_1(B1): CanBreak1, continue_0(C0): CanContinue0)
}

define_loop_signal! {
    4
    L4SignalNoOuter;
    L4SignalNoOuterNoBreakLocal;
    L4SignalNoOuterNoContinueParent;
    L4SignalNoOuterNoBreakLocalNoContinueParent;

    L4Signal(L3Signal)
        (break_4(B4): CanBreak4, continue_3(C3): CanContinue3)
        (break_3(B3): CanBreak3, continue_2(C2): CanContinue2)
        (break_2(B2): CanBreak2, continue_1(C1): CanContinue1)
        (break_1(B1): CanBreak1, continue_0(C0): CanContinue0)
}

define_loop_signal! {
    5
    L5SignalNoOuter;
    L5SignalNoOuterNoBreakLocal;
    L5SignalNoOuterNoContinueParent;
    L5SignalNoOuterNoBreakLocalNoContinueParent;

    L5Signal(L4Signal)
        (break_5(B5): CanBreak5, continue_4(C4): CanContinue4)
        (break_4(B4): CanBreak4, continue_3(C3): CanContinue3)
        (break_3(B3): CanBreak3, continue_2(C2): CanContinue2)
        (break_2(B2): CanBreak2, continue_1(C1): CanContinue1)
        (break_1(B1): CanBreak1, continue_0(C0): CanContinue0)
}

define_loop_signal! {
    6
    L6SignalNoOuter;
    L6SignalNoOuterNoBreakLocal;
    L6SignalNoOuterNoContinueParent;
    L6SignalNoOuterNoBreakLocalNoContinueParent;

    L6Signal(L5Signal)
        (break_6(B6): CanBreak6, continue_5(C5): CanContinue5)
        (break_5(B5): CanBreak5, continue_4(C4): CanContinue4)
        (break_4(B4): CanBreak4, continue_3(C3): CanContinue3)
        (break_3(B3): CanBreak3, continue_2(C2): CanContinue2)
        (break_2(B2): CanBreak2, continue_1(C1): CanContinue1)
        (break_1(B1): CanBreak1, continue_0(C0): CanContinue0)
}

define_loop_signal! {
    7
    L7SignalNoOuter;
    L7SignalNoOuterNoBreakLocal;
    L7SignalNoOuterNoContinueParent;
    L7SignalNoOuterNoBreakLocalNoContinueParent;

    L7Signal(L6Signal)
        (break_7(B7): CanBreak7, continue_6(C6): CanContinue6)
        (break_6(B6): CanBreak6, continue_5(C5): CanContinue5)
        (break_5(B5): CanBreak5, continue_4(C4): CanContinue4)
        (break_4(B4): CanBreak4, continue_3(C3): CanContinue3)
        (break_3(B3): CanBreak3, continue_2(C2): CanContinue2)
        (break_2(B2): CanBreak2, continue_1(C1): CanContinue1)
        (break_1(B1): CanBreak1, continue_0(C0): CanContinue0)
}

define_loop_signal! {
    8
    L8SignalNoOuter;
    L8SignalNoOuterNoBreakLocal;
    L8SignalNoOuterNoContinueParent;
    L8SignalNoOuterNoBreakLocalNoContinueParent;

    L8Signal(L7Signal)
        (break_8(B8): CanBreak8, continue_7(C7): CanContinue7)
        (break_7(B7): CanBreak7, continue_6(C6): CanContinue6)
        (break_6(B6): CanBreak6, continue_5(C5): CanContinue5)
        (break_5(B5): CanBreak5, continue_4(C4): CanContinue4)
        (break_4(B4): CanBreak4, continue_3(C3): CanContinue3)
        (break_3(B3): CanBreak3, continue_2(C2): CanContinue2)
        (break_2(B2): CanBreak2, continue_1(C1): CanContinue1)
        (break_1(B1): CanBreak1, continue_0(C0): CanContinue0)
}

define_loop_signal! {
    9
    L9SignalNoOuter;
    L9SignalNoOuterNoBreakLocal;
    L9SignalNoOuterNoContinueParent;
    L9SignalNoOuterNoBreakLocalNoContinueParent;

    L9Signal(L8Signal)
        (break_9(B9): CanBreak9, continue_8(C8): CanContinue8)
        (break_8(B8): CanBreak8, continue_7(C7): CanContinue7)
        (break_7(B7): CanBreak7, continue_6(C6): CanContinue6)
        (break_6(B6): CanBreak6, continue_5(C5): CanContinue5)
        (break_5(B5): CanBreak5, continue_4(C4): CanContinue4)
        (break_4(B4): CanBreak4, continue_3(C3): CanContinue3)
        (break_3(B3): CanBreak3, continue_2(C2): CanContinue2)
        (break_2(B2): CanBreak2, continue_1(C1): CanContinue1)
        (break_1(B1): CanBreak1, continue_0(C0): CanContinue0)
}
