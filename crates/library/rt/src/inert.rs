//! Inert execution and typecheck helpers emitted during lowering.

use colorless_core::{Colored, Colorize, IntoColored};

#[doc(hidden)]
#[inline(always)]
pub fn colorize<C, D>(d: impl FnOnce() -> D) -> impl Colored<C, Output = D::Output>
where
    D: Colorize<C>,
{
    d().colorize()
}

// having _c return O is problematic with opaque types, not is with generic types
// #[doc(hidden)]
// pub fn never_execute_inert<O, Oz, C, D: Colored<C, Output = O>>(_c: impl FnOnce() -> Oz) -> D {
//     unsafe { core::hint::unreachable_unchecked() }
// }
#[doc(hidden)]
#[inline(always)]
pub fn never_execute_inert<O, C, D: Colored<C, Output = O>>(_c: impl FnOnce() -> O) -> D {
    unsafe { core::hint::unreachable_unchecked() }
}

/// Compiler intrinsic marker used by procedural macros during lowering.
#[doc(hidden)]
#[inline(always)]
pub fn resolve_unreachable<C, Co: IntoColored<C>>(_s: Co) -> UnreachableResolve<Co::Output> {
    unreachable!()
}

#[doc(hidden)]
pub struct UnreachableResolve<T> {
    pub resolve: T,
}

#[macro_export]
macro_rules! keep_alive {
    ($($ident:ident),*) => {
        $(
            #[allow(unused)]
            let $ident = $ident;
        )*
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use colorless_core::{Identity, IdentityExt};

    #[test]
    fn test_unreachable_resolve() {
        let r = UnreachableResolve { resolve: 42 };
        assert_eq!(r.resolve, 42);
    }

    #[test]
    fn test_colorize() {
        let colored = colorize::<Identity, _>(|| 123);
        assert_eq!(colored.sync_(), 123);
    }

    #[test]
    fn test_keep_alive() {
        let a = 1;
        let b = 2;
        keep_alive!(a, b);
        assert_eq!(a + b, 3);
    }
}
