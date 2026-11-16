// -----------------------------------------------------------------------------
// Break Capability Traits (Levels 0..9)
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

// -----------------------------------------------------------------------------
// Continue Capability Traits (Levels 0..8)
// -----------------------------------------------------------------------------

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

// -----------------------------------------------------------------------------
// Return Capability Trait
// -----------------------------------------------------------------------------

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
