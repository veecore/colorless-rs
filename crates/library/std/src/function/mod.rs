//! Function traits, closures, and higher-order colored combinators.

use colorless_macros::colorless;

#[colorless(C)]
#[must_use = "colored closures are lazy and do nothing unless called"]
pub trait ColoredFn<Args>: ColoredFnMut<Args, C> {
    fn call(&self, args: Args) -> Self::Output;
}

#[colorless(C)]
#[must_use = "colored closures are lazy and do nothing unless called"]
pub trait ColoredFnMut<Args>: ColoredFnOnce<Args, C> {
    fn call_mut(&mut self, args: Args) -> Self::Output;
}

#[colorless(C)]
#[must_use = "colored closures are lazy and do nothing unless called"]
pub trait ColoredFnOnce<Args> {
    type Output;

    fn call_once(self, args: Args) -> Self::Output;
}

mod impls {
    use super::*;
    use core::ops::DerefMut;

    #[colorless(C)]
    impl<A, F> ColoredFn<A> for &F
    where
        F: ColoredFn<A, C>,
    {
        fn call(&self, args: A) -> Self::Output {
            F::call(*self, args).resolve
        }
    }

    #[colorless(C)]
    impl<A, F> ColoredFnMut<A> for &F
    where
        F: ColoredFn<A, C>,
    {
        fn call_mut(&mut self, args: A) -> Self::Output {
            F::call(*self, args).resolve
        }
    }

    #[colorless(C)]
    impl<A, F> ColoredFnOnce<A> for &F
    where
        F: ColoredFn<A, C>,
    {
        type Output = F::Output;

        fn call_once(self, args: A) -> Self::Output {
            F::call(self, args).resolve
        }
    }

    #[colorless(C)]
    impl<A, F> ColoredFnMut<A> for &mut F
    where
        F: ColoredFn<A, C>,
    {
        fn call_mut(&mut self, s: A) -> Self::Output {
            F::call_mut(self.deref_mut(), s).resolve
        }
    }

    #[colorless(C)]
    impl<A, F> ColoredFnOnce<A> for &mut F
    where
        F: ColoredFn<A, C>,
    {
        type Output = F::Output;

        fn call_once(self, args: A) -> Self::Output {
            F::call_mut(self, args).resolve
        }
    }
}

pub use adapters::*;

mod adapters {

    // -------------------------------- MAP --------------------------------

    use crate::function::{ColoredFn, ColoredFnMut, ColoredFnOnce};
    use colorless_macros::colorless;

    #[must_use = "colored closures are lazy and do nothing unless called"]
    pub struct Map<A, B> {
        a: A,
        b: B,
    }

    pub fn map<A, B>(a: A, b: B) -> Map<A, B> {
        Map { a, b }
    }

    #[colorless(C)]
    impl<A, B, Args> ColoredFn<Args> for Map<A, B>
    where
        A: ColoredFn<Args, C>,
        B: ColoredFn<A::Output, C>,
    {
        #[inline]
        fn call(&self, args: Args) -> Self::Output {
            self.b.call(self.a.call(args).resolve).resolve
        }
    }

    #[colorless(C)]
    impl<A, B, Args> ColoredFnMut<Args> for Map<A, B>
    where
        A: ColoredFnMut<Args, C>,
        B: ColoredFnMut<A::Output, C>,
    {
        #[inline]
        fn call_mut(&mut self, args: Args) -> Self::Output {
            // fixme: lowering is ruining disjoint borrow
            let a = &mut self.a;
            let b = &mut self.b;
            b.call_mut(a.call_mut(args).resolve).resolve
        }
    }

    #[colorless(C)]
    impl<A, B, Args> ColoredFnOnce<Args> for Map<A, B>
    where
        A: ColoredFnOnce<Args, C>,
        B: ColoredFnOnce<A::Output, C>,
    {
        type Output = B::Output;

        #[inline]
        fn call_once(self, args: Args) -> Self::Output {
            self.b.call_once(self.a.call_once(args).resolve).resolve
        }
    }

    // -------------------------------- IdentityFn --------------------------------
    #[must_use = "colored closures are lazy and do nothing unless called"]
    pub struct IdentityFn;

    #[inline]
    pub fn identity_fn() -> IdentityFn {
        IdentityFn
    }

    #[colorless]
    impl<Args> ColoredFn<Args> for IdentityFn {
        #[inline]
        fn call(&self, s: Args) -> Self::Output {
            s
        }
    }

    #[colorless]
    impl<Args> ColoredFnMut<Args> for IdentityFn {
        #[inline]
        fn call_mut(&mut self, s: Args) -> Self::Output {
            s
        }
    }

    #[colorless]
    impl<Args> ColoredFnOnce<Args> for IdentityFn {
        type Output = Args;

        #[inline]
        fn call_once(self, s: Args) -> Self::Output {
            s
        }
    }

    // -------------------------------- DiscardFn --------------------------------

    #[must_use = "colored closures are lazy and do nothing unless called"]
    pub struct Discard;

    #[inline]
    pub fn discard() -> Discard {
        Discard
    }

    #[colorless]
    impl<Args> ColoredFn<Args> for Discard {
        fn call(&self, _s: Args) -> Self::Output {}
    }

    #[colorless]
    impl<Args> ColoredFnMut<Args> for Discard {
        #[inline]
        fn call_mut(&mut self, _: Args) -> Self::Output {}
    }

    #[colorless]
    impl<Args> ColoredFnOnce<Args> for Discard {
        type Output = ();

        #[inline]
        fn call_once(self, _args: Args) -> Self::Output {}
    }
}

pub use bridge::*;

// TODO: COVER MORE
mod bridge {
    use crate::function::{ColoredFn, ColoredFnMut, ColoredFnOnce};
    use colorless_core::{Async, Colored, Identity};
    use colorless_macros::colorless;

    // -------------------------------- sync fn --------------------------------

    #[must_use = "colored closures are lazy and do nothing unless called"]
    pub struct Closure<F>(F);

    #[inline]
    pub fn from_fn<F>(f: F) -> Closure<F> {
        Closure(f)
    }

    #[colorless(color = Identity)]
    impl<R, F, Args> ColoredFn<Args> for Closure<F>
    where
        F: Fn(Args) -> R,
    {
        #[inline]
        fn call(&self, s: Args) -> Self::Output {
            self.0(s)
        }
    }

    #[colorless(color = Identity)]
    impl<R, F, Args> ColoredFnMut<Args> for Closure<F>
    where
        F: FnMut(Args) -> R,
    {
        #[inline]
        fn call_mut(&mut self, s: Args) -> Self::Output {
            self.0(s)
        }
    }

    #[colorless(color = Identity)]
    impl<R, F, Args> ColoredFnOnce<Args> for Closure<F>
    where
        F: FnOnce(Args) -> R,
    {
        type Output = R;

        #[inline]
        fn call_once(self, args: Args) -> Self::Output {
            self.0(args)
        }
    }

    // -------------------------------- async fn --------------------------------

    #[must_use = "colored closures are lazy and do nothing unless called"]
    pub struct AsyncClosure<F>(F);

    #[inline]
    pub fn from_async_fn<F>(f: F) -> AsyncClosure<F> {
        AsyncClosure(f)
    }

    #[colorless(color = Async)]
    impl<R, F, Args> ColoredFn<Args> for AsyncClosure<F>
    where
        F: AsyncFn(Args) -> R,
    {
        #[inline]
        async fn call(&self, s: Args) -> Self::Output {
            self.0(s).await
        }
    }

    #[colorless(color = Async)]
    impl<R, F, Args> ColoredFnMut<Args> for AsyncClosure<F>
    where
        F: AsyncFnMut(Args) -> R,
    {
        #[inline]
        async fn call_mut(&mut self, s: Args) -> Self::Output {
            self.0(s).await
        }
    }

    #[colorless(color = Async)]
    impl<R, F, Args> ColoredFnOnce<Args> for AsyncClosure<F>
    where
        F: AsyncFnOnce(Args) -> R,
    {
        type Output = R;

        #[inline]
        async fn call_once(self, args: Args) -> Self::Output {
            self.0(args).await
        }
    }

    // -------------------------------- ColoredClosure --------------------------------

    #[must_use = "colored closures are lazy and do nothing unless called"]
    pub struct ColoredClosure<F>(F);

    #[inline]
    pub fn from_colored_fn<F>(f: F) -> ColoredClosure<F> {
        ColoredClosure(f)
    }

    #[colorless(C)]
    impl<R, F, Args> ColoredFn<Args> for ColoredClosure<F>
    where
        F: Fn(Args) -> R,
        R: Colored<C>,
    {
        #[inline]
        fn call(&self, s: Args) -> Self::Output {
            let a = &self.0;
            a(s).resolve
        }
    }

    #[colorless(C)]
    impl<R, F, Args> ColoredFnMut<Args> for ColoredClosure<F>
    where
        F: FnMut(Args) -> R,
        R: Colored<C>,
    {
        #[inline]
        fn call_mut(&mut self, s: Args) -> Self::Output {
            let a = &mut self.0;
            a(s).resolve
        }
    }

    #[colorless(C)]
    impl<R, F, Args> ColoredFnOnce<Args> for ColoredClosure<F>
    where
        F: FnOnce(Args) -> R,
        R: Colored<C>,
    {
        type Output = R::Output;

        #[inline]
        fn call_once(self, args: Args) -> Self::Output {
            self.0(args).resolve
        }
    }
}
