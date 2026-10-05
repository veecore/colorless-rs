// -----------------------------------------------------------------------------
// Type Definitions
// -----------------------------------------------------------------------------

use core::convert::Infallible;

/// Placeholder for the never type.
///
/// This is used for signal payloads that cannot occur at a particular
/// instantiation. It can be replaced with `!` once the crate no longer needs
/// `Infallible` for compatibility with the supported compiler versions.
pub type Never = Infallible;

// -----------------------------------------------------------------------------
// Control-Flow Capability Traits & Free Polymorphic Methods
// -----------------------------------------------------------------------------

macro_rules! define_break_traits {
    ($(($trait_name:ident, $assoc:ident, $method:ident)),* $(,)?) => {
        $(
            #[doc = concat!("Capability to break from loop nesting level `", stringify!($method), "`.")]
            pub trait $trait_name {
                type $assoc;
                fn $method(b: Self::$assoc) -> Self;
            }

            #[doc = concat!("Polymorphic helper to break from loop nesting level `", stringify!($method), "`.")]
            #[inline(always)]
            pub fn $method<C: $trait_name>(b: C::$assoc) -> C {
                C::$method(b)
            }
        )*
    };
}

define_break_traits! {
    (CanBreak0, B0, break_0),
    (CanBreak1, B1, break_1),
    (CanBreak2, B2, break_2),
    (CanBreak3, B3, break_3),
    (CanBreak4, B4, break_4),
    (CanBreak5, B5, break_5),
    (CanBreak6, B6, break_6),
    (CanBreak7, B7, break_7),
    (CanBreak8, B8, break_8),
    (CanBreak9, B9, break_9),
}

macro_rules! define_continue_traits {
    ($(($trait_name:ident, $assoc:ident, $method:ident)),* $(,)?) => {
        $(
            #[doc = concat!("Capability to continue loop nesting level via `", stringify!($method), "`.")]
            pub trait $trait_name {
                type $assoc;

                fn $method(c: Self::$assoc) -> Self;
            }

            #[doc = concat!("Polymorphic helper to continue loop nesting level via `", stringify!($method), "`.")]
            #[inline(always)]
            pub fn $method<C: $trait_name>(c: C::$assoc) -> C {
                C::$method(c)
            }
        )*
    };
}

define_continue_traits! {
    (CanContinue0, C0, continue_0),
    (CanContinue1, C1, continue_1),
    (CanContinue2, C2, continue_2),
    (CanContinue3, C3, continue_3),
    (CanContinue4, C4, continue_4),
    (CanContinue5, C5, continue_5),
    (CanContinue6, C6, continue_6),
    (CanContinue7, C7, continue_7),
    (CanContinue8, C8, continue_8),
}

/// Capability to return from the enclosing function boundary.
pub trait CanReturn {
    type R;
    fn return_(r: Self::R) -> Self;
}

/// Polymorphic helper to return from the enclosing function boundary.
#[inline(always)]
pub fn return_<C: CanReturn>(r: C::R) -> C {
    C::return_(r)
}

// -----------------------------------------------------------------------------
// Level 0 (Outermost Loop Level)
// -----------------------------------------------------------------------------

/// Control-flow signal at the outermost loop level.
///
/// A level-0 signal has nowhere else to propagate to except the function
/// boundary. [`L0Signal::BreakLocal`] represents a break from the outermost
/// loop, while [`L0Signal::Return`] represents a function return.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
#[must_use]
pub enum L0Signal<B = Never, R = Never> {
    /// Break from the current (outermost) loop with `B`.
    BreakLocal(B),

    /// Return from the enclosing function with `R`.
    Return(R),
}

impl<B, R> L0Signal<B, R> {
    /// Creates a signal that breaks from the outermost loop.
    #[inline(always)]
    pub const fn break_0(value: B) -> Self {
        Self::BreakLocal(value)
    }

    /// Creates a signal that returns from the enclosing function.
    #[inline(always)]
    pub const fn return_(value: R) -> Self {
        Self::Return(value)
    }

    /// Alias for [`L0Signal::break_0`].
    #[inline(always)]
    pub const fn break_local(value: B) -> Self {
        Self::BreakLocal(value)
    }
}

/// A level-0 signal that cannot return from the function.
pub type L0SignalNoReturn<B> = L0Signal<B>;

/// A level-0 signal that cannot break from its local loop.
pub type L0SignalNoBreakLocal<R> = L0Signal<Never, R>;

/// A level-0 signal that can neither return nor break locally.
pub type L0SignalNoReturnNoBreakLocal = L0Signal;

impl<B0, R> CanBreak0 for L0Signal<B0, R> {
    type B0 = B0;

    #[inline(always)]
    fn break_0(b: Self::B0) -> Self {
        Self::break_0(b)
    }
}

impl<B0, R> CanReturn for L0Signal<B0, R> {
    type R = R;

    #[inline(always)]
    fn return_(r: Self::R) -> Self {
        Self::return_(r)
    }
}

// -----------------------------------------------------------------------------
// ABSTRACT Levels 1..N
// -----------------------------------------------------------------------------

// This is also to disable pinning... I discovered this bug late
// WE CAN REMOVE THE DECLARED L1-L9 BECAUSE OF THIS ACTUALLY
// We can alias L1Signal...L2Signal with this and keep our Capability traits
/// Control-flow signal at some loop nesting level
///
/// The signal can target the current loop or propagate to an enclosing
/// loop or function boundary through [`Self::Outer`].
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
#[must_use]
pub enum LnSignal<Bn, Cp, Ln_1> {
    /// Break from the current loop with the associated value.
    BreakLocal(Bn),

    /// Continue the immediately enclosing loop.
    ContinueParent(Cp),

    /// Propagate a signal to an enclosing control-flow scope.
    Outer(Ln_1),
}

impl<Bn, Cp, Ln_1> LnSignal<Bn, Cp, Ln_1> {
    /// Creates a signal that breaks from the current loop.
    #[inline(always)]
    pub const fn break_local(b: Bn) -> Self {
        Self::BreakLocal(b)
    }

    /// Creates a signal that continues the immediately enclosing loop.
    #[inline(always)]
    pub const fn continue_parent(c: Cp) -> Self {
        Self::ContinueParent(c)
    }

    /// Creates a signal that returns from the enclosing function.
    #[inline(always)]
    pub const fn outer(l: Ln_1) -> Self {
        Self::Outer(l)
    }
}

/// A signal type that cannot break from the current loop.
pub type LnSignalNoBreakLocal<Cp, Ln_1> = LnSignal<Never, Cp, Ln_1>;
// TODO: DOC
pub type LnSignalNoContinueParent<Bn, Ln_1> = LnSignal<Bn, Never, Ln_1>;
pub type LnSignalNoBreakLocalNoContinueParent<Ln_1> = LnSignal<Never, Never, Ln_1>;

// -----------------------------------------------------------------------------
// Levels 1..N
// -----------------------------------------------------------------------------

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
        /// A signal type that cannot propagate outward and cannot break
        /// from the current loop.
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
    L5ignalNoOuter;
    L51SignalNoOuterNoBreakLocal;
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

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    // These tests intentionally model control transfer using one-iteration
    // loops, mirroring the generated CPS shape. Their loops are structural,
    // not accidental dead code.
    #![allow(clippy::never_loop)]

    use super::*;

    // NOTE: THESE TEST TYPE INFERENCE

    #[test]
    fn local_break() {
        // loop {
        //     break 1
        // }
        let signal;
        loop {
            signal = break_0(42);
            break;
        }
        match signal {
            L0SignalNoReturn::BreakLocal(value) => assert_eq!(value, 42),
            L0SignalNoReturn::Return(_) => unreachable!(),
        }
    }

    #[test]
    #[ignore = "will never return"]
    fn parent_continue() {
        // 'a: loop {
        //     loop {
        //         continue 'a;
        //     }
        // }
        let mut signal;
        loop {
            loop {
                signal = continue_0(());
                break;
            }
            match signal {
                L1SignalNoOuterNoBreakLocal::ContinueParent(_) => continue,
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn break_to_parent() {
        // 'a: loop {
        //     loop {
        //         break 'a 42;
        //     }
        // };
        let signal_a;
        loop {
            let signal_b;
            loop {
                signal_b = break_0(42);
                break;
            }

            match signal_b {
                LnSignalNoBreakLocalNoContinueParent::Outer(parent) => {
                    signal_a = parent;
                    break;
                }
                _ => unreachable!(),
            }
        }

        match signal_a {
            L0SignalNoReturn::BreakLocal(value) => {
                assert_eq!(value, 42);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn break_to_outermost_loop() {
        // 'a: loop {
        //     'b: loop {
        //         'c: loop {
        //             break 'a 42;
        //         }
        //     }
        // }
        let signal_a;
        loop {
            let signal_b;
            loop {
                let signal_c;
                loop {
                    signal_c = break_0(42);
                    break;
                }

                match signal_c {
                    LnSignalNoBreakLocalNoContinueParent::Outer(parent) => {
                        signal_b = parent;
                        break;
                    }
                    _ => unreachable!(),
                }
            }

            match signal_b {
                LnSignalNoBreakLocalNoContinueParent::Outer(parent) => {
                    signal_a = parent;
                    break;
                }
                _ => unreachable!(),
            }
        }

        match signal_a {
            L0SignalNoReturn::BreakLocal(x) => assert_eq!(x, 42),
            _ => unreachable!(),
        }
    }

    #[test]
    fn return_to_function_boundary() {
        // 'a: loop {
        //     'b: loop {
        //         'c: loop {
        //             return 42;
        //         }
        //     }
        // }

        let signal_a;
        loop {
            let signal_b;
            loop {
                let signal_c;
                loop {
                    signal_c = return_(42);
                    break;
                }

                match signal_c {
                    LnSignalNoBreakLocalNoContinueParent::Outer(parent) => {
                        signal_b = parent;
                        break;
                    }
                    _ => unreachable!(),
                }
            }

            match signal_b {
                LnSignalNoBreakLocalNoContinueParent::Outer(parent) => {
                    signal_a = parent;
                    break;
                }
                _ => unreachable!(),
            }
        }

        match signal_a {
            L0SignalNoBreakLocal::Return(x) => assert_eq!(x, 42),
            _ => unreachable!(),
        }
    }

    #[test]
    fn local_break_does_not_propagate() {
        // loop {
        //     loop {
        //          ..
        //              ..
        //                  loop {
        //                      break 42
        //                  }
        //              ..
        //          ..
        //     }
        // }

        let signal = L9Signal::break_9(42);

        match signal {
            L9SignalNoOuterNoContinueParent::BreakLocal(value) => assert_eq!(value, 42),
            _ => unreachable!(),
        }
    }

    #[test]
    #[ignore = "will never return"]
    fn continue_parent_cascade() {
        // Simulates:
        //
        // let mut total = 0;
        // 'outer: loop {
        //     total += 1;
        //     loop {
        //         loop {
        //              if total < 42 {
        //                  continue 'outer;
        //              }
        //         }
        //     }
        // }
        //
        // The signal should cross two loop boundaries before reaching the
        // loop that owns it.

        let mut total = 0;

        loop {
            total += 1;
            let signal_b;
            loop {
                let signal_c;
                loop {
                    if total < 42 {
                        signal_c = L2Signal::continue_0(());
                        break;
                    }
                }

                match signal_c {
                    LnSignalNoBreakLocalNoContinueParent::Outer(value) => {
                        signal_b = value;
                        break;
                    }
                    _ => unreachable!(),
                }
            }

            match signal_b {
                L1SignalNoOuterNoBreakLocal::ContinueParent(_) => continue,
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn polymorphic_break_helpers() {
        fn produce_break_0<C: CanBreak0<B0 = i32>>(val: i32) -> C {
            break_0(val)
        }

        fn produce_break_1<C: CanBreak1<B1 = &'static str>>(val: &'static str) -> C {
            break_1(val)
        }

        let l0: L0Signal<i32, ()> = produce_break_0(10);
        assert_eq!(l0, L0Signal::BreakLocal(10));

        let l1: L1Signal<&'static str, Never, i32, ()> = produce_break_0(20);
        assert_eq!(l1, L1Signal::Outer(L0Signal::BreakLocal(20)));

        let l1_local: L1Signal<&'static str, i32, ()> = produce_break_1("hello");
        assert_eq!(l1_local, L1Signal::BreakLocal("hello"));

        let l2_outer: L2Signal<(), Never, &'static str, i32, ()> = produce_break_1("world");
        assert_eq!(l2_outer, L2Signal::Outer(L1Signal::BreakLocal("world")));
    }

    #[test]
    fn polymorphic_continue_helpers() {
        fn produce_continue_0<C: CanContinue0>(c: C::C0) -> C {
            continue_0(c)
        }

        let l1: L1Signal<(), (), (), ()> = produce_continue_0(());
        assert_eq!(l1, L1Signal::ContinueParent(()));

        let l2: L2Signal<(), (), (), (), ()> = produce_continue_0(());
        assert_eq!(l2, L2Signal::Outer(L1Signal::ContinueParent(())));
    }

    #[test]
    fn polymorphic_return_helper() {
        fn produce_return<C: CanReturn<R = i32>>(r: i32) -> C {
            return_(r)
        }

        let l0: L0Signal<(), i32> = produce_return(99);
        assert_eq!(l0, L0Signal::Return(99));

        let l1: L1Signal<(), Never, (), i32> = produce_return(100);
        assert_eq!(l1, L1Signal::Outer(L0Signal::Return(100)));

        let l3: L3Signal<(), Never, (), Never, (), Never, (), i32> = produce_return(101);
        assert_eq!(
            l3,
            L3Signal::Outer(L2Signal::Outer(L1Signal::Outer(L0Signal::Return(101))))
        );
    }
}
