use crate::{Colored, Cps, Linear, callbacks::ColoredCallFold};
use core::ops::ControlFlow;

/// # Iteration & State-Threading Loops
///
/// In direct-style Rust, loops are an imperative primitive:
///
/// ```rust,ignore
/// while condition {
///     step();
/// }
/// ```
///
/// In asynchronous or generic execution contexts, however, an iteration step may suspend
/// across a yield point. A native `while` or `loop` cannot suspend in-place without a coroutine
/// state machine.
///
/// In Continuation-Passing Style, iteration is fundamentally **tail recursion**: each iteration
/// evaluates a step and returns either a termination signal or the previous computation.
///
/// ### The Return-Position Signal
///
/// Colorless models iteration through [`ColoredLoop`]:
///
/// ```rust,ignore
/// pub trait ColoredLoop<C>: Sized {
///     type Break;
///     fn repeat(self) -> impl Colored<C, Output = ControlFlow<Self::Break, Self>>;
/// }
/// ```
///
/// Rather than requiring an external driver to inspect and manipulate mutable loop variables,
/// the loop step consumes `self` and returns a control-flow signal:
/// - `ControlFlow::Break(result)`: terminates the loop and yields the final value.
/// - `ControlFlow::Continue(next_state)`: provides the state for the subsequent iteration.
///
/// In synchronous contexts ([`Identity`](crate::Identity)), this compiles to an unboxed
/// tail-recursive jump; in [`Async`](crate::Async), it loops over the unboxed future in
/// a single pinned frame.
///
/// ### Closure Recursion vs. Named Loop Structs
///
/// In handwritten code, closures cannot easily return themselves (`ControlFlow::Continue(self)`)
/// because Rust closures lack some sort of `self` identifier.
///
/// When writing loops manually, defining a local `struct LoopStep<'a>` inside the function body
/// provides a clean, zero-cost implementation of [`ColoredLoop`] where `self` is natively available.
///
/// Procedural macros accompanying this crate requires state annotation to use the codegen method
pub trait Loop: Linear + Sized {
    /// Drives a [`ColoredLoop`] step to completion, looping until it returns
    /// [`ControlFlow::Break`].
    ///
    /// In synchronous colors this compiles to an ordinary Rust `loop`.
    /// In asynchronous colors the loop body runs inside a single pinned frame,
    /// suspending at each `poll` boundary without allocating.
    ///
    /// Prefer [`Loop::fold`] when the loop state is a plain value; use
    /// `loop_` directly when the step is a hand-written [`ColoredLoop`] impl.
    fn loop_<F>(f: F) -> impl Colored<Self, Output = F::Break>
    where
        F: ColoredLoop<Self>;

    /// Drives a stateful loop where each iteration receives the accumulated
    /// state and returns either [`ControlFlow::Continue`] with the next state
    /// or [`ControlFlow::Break`] with the final value.
    ///
    /// This is the color-generic equivalent of a `while`/`loop` with an
    /// explicit accumulator, generalized to work across yield points.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use colorless_core::{Color, Colored, Async, AsyncExt, OneOf2};
    /// # use colorless_core::test_util::block_on_test;
    /// use core::ops::ControlFlow;
    ///
    /// fn sum_to<C: Color>(n: i32) -> impl Colored<C, Output = i32> {
    ///     C::fold((0_i32, 0_i32), move |(sum, i)| {
    ///         if i < n {
    ///             <C as OneOf2>::arm_0(C::ready(ControlFlow::Continue((sum + i, i + 1))))
    ///         } else {
    ///             <C as OneOf2>::arm_1(C::ready(ControlFlow::Break(sum)))
    ///         }
    ///     })
    /// }
    ///
    /// assert_eq!(block_on_test(sum_to::<Async>(5).async_()), 10);
    /// ```
    #[inline(always)]
    fn fold<S, F>(state: S, step: F) -> impl Colored<Self, Output = F::Break>
    where
        F: ColoredCallFold<Self, S>,
        Self: Cps,
    {
        Self::loop_((state, step))
    }

    #[cfg(any())]
    #[inline(always)]
    fn fold_mut<S, K, F>(state_mut: S, state: K, step: F) -> impl Colored<Self, Output = F::Break>
    where
        F: ColoredCallFoldMut<Self, S, K>,
        Self: Cps + LocalBorrow,
    {
        Self::borrow(state_mut, LoopMutScope(state, step))
    }
}

/// A single loop-step type that can repeat itself inside a [`Loop`] context.
///
/// Each call to [`repeat`](ColoredLoop::repeat) consumes `self` and returns a
/// computation that resolves to either:
/// - `ControlFlow::Continue(next_self)` — another iteration begins with the
///   new `self` as the loop state.
/// - `ControlFlow::Break(result)` — the loop terminates and [`Loop::loop_`]
///   returns `result`.
///
/// # Implementing by hand
///
/// Define a local struct whose fields are the loop variables and implement
/// `ColoredLoop<C>` on it. The `self` parameter gives natural access to all
/// captured state.
///
/// ```rust
/// # use colorless_core::{Color, Colored, ColoredLoop, Loop, Identity, IdentityExt, OneOf2};
/// use core::ops::ControlFlow;
///
/// struct Counter { remaining: u32, sum: u32 }
///
/// impl<C: Color> ColoredLoop<C> for Counter {
///     type Break = u32;
///
///     fn repeat(mut self) -> impl Colored<C, Output = ControlFlow<u32, Self>> {
///         if self.remaining == 0 {
///             <C as OneOf2>::arm_1(C::ready(ControlFlow::Break(self.sum)))
///         } else {
///             self.sum += self.remaining;
///             self.remaining -= 1;
///             <C as OneOf2>::arm_0(C::ready(ControlFlow::Continue(self)))
///         }
///     }
/// }
///
/// let result = Identity::loop_(Counter { remaining: 5, sum: 0 }).sync_();
/// assert_eq!(result, 15); // 5+4+3+2+1
/// ```
pub trait ColoredLoop<C>: Sized {
    /// The value produced when the loop terminates via [`ControlFlow::Break`].
    type Break;

    /// Executes one loop iteration.
    ///
    /// Returns a computation that resolves to `ControlFlow::Continue(self)`
    /// to continue looping with updated state, or `ControlFlow::Break(value)`
    /// to exit with a final result.
    fn repeat(self) -> impl Colored<C, Output = ControlFlow<Self::Break, Self>>;
}

mod impls {
    use crate::callbacks::{ColoredCallFold, ColoredCallFoldMut};
    use crate::{BorrowScope, Colored, ColoredLoop, Cps, Loop};
    use core::ops::ControlFlow;

    impl<C: Cps, State, F> ColoredLoop<C> for (State, F)
    where
        F: ColoredCallFold<C, State>,
    {
        type Break = F::Break;

        #[inline(always)]
        fn repeat(mut self) -> impl Colored<C, Output = ControlFlow<Self::Break, Self>> {
            let f = (self.1)(self.0);
            C::map(f, move |x| match x {
                ControlFlow::Continue(c) => ControlFlow::Continue((c, self.1)),
                ControlFlow::Break(b) => ControlFlow::Break(b),
            })
        }
    }

    #[allow(dead_code)]
    pub(super) struct LoopMutScope<K_, F_>(pub(super) K_, pub(super) F_);

    impl<C, S, K_, F_> BorrowScope<C, S> for LoopMutScope<K_, F_>
    where
        F_: ColoredCallFoldMut<C, S, K_>,
        C: Loop + Cps,
    {
        type Resume = F_::Break;

        #[inline(always)]
        fn resume_mut(mut self, var: &mut S) -> impl Colored<C, Output = Self::Resume> {
            C::loop_((self.0, move |k| self.1(var, k)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Async, AsyncExt, Color, OneOf2, Ready,
        test_util::{block_on_test, fetch},
    };

    #[test]
    fn manual_impl() {
        /*
        fn ex8_sync(x: &mut i32) -> i32 {
            let mut i = 0;
            while i < 10 {
                *x += fetch(i).resolve;
                i += 1;
            }
            *x
        }
        */

        struct LoopBody<'a> {
            x: &'a mut i32,
            i: i32,
        }

        impl<'a, C: Color> ColoredLoop<C> for LoopBody<'a> {
            type Break = &'a mut i32;

            #[inline(always)]
            fn repeat(mut self) -> impl Colored<C, Output = ControlFlow<Self::Break, Self>> {
                if self.i < 10 {
                    <C as OneOf2>::arm_0(C::map(fetch(self.i), move |f| {
                        *self.x += f;
                        self.i += 1;
                        ControlFlow::Continue(self)
                    }))
                } else {
                    // we could already evaluate here but that wouldn't be exact with the
                    // provided code so we thread
                    <C as OneOf2>::arm_1(C::ready(ControlFlow::Break(self.x)))
                }
            }
        }

        fn func<C: Color + Loop>(x: &mut i32) -> impl Colored<C, Output = i32> {
            let l = C::loop_(LoopBody { x, i: 0 });
            C::map(l, move |x| *x)
        }

        let mut x_async = 0;
        let res_async = block_on_test(func(&mut x_async).async_());
        assert_eq!(res_async, 45);
        assert_eq!(x_async, 45);
    }

    #[test]
    fn fold_loop() {
        fn func<C: Color + Loop>(x: &mut i32) -> impl Colored<C, Output = i32> {
            C::wrap_flatten(move || {
                let i = 0;
                let f = C::fold((x, i), |(x, mut i)| {
                    if i < 10 {
                        let a = C::map(fetch(i), move |o| {
                            *x += o;
                            i += 1;
                            ControlFlow::Continue((x, i))
                        });
                        <C as OneOf2>::arm_0(a)
                    } else {
                        <C as OneOf2>::arm_1(C::ready(ControlFlow::Break(x)))
                    }
                });

                C::map(f, move |x| *x)
            })
        }

        let mut x_async = 0;
        let res_async = block_on_test(func(&mut x_async).async_());
        assert_eq!(res_async, 45);
        assert_eq!(x_async, 45);
    }

    #[test]
    fn fn_mut() {
        let mut x = 0;
        let f = Async::fold((), move |_| {
            x += 1;
            if x == 2 {
                return <Async as OneOf2>::arm_0(Async::ready(ControlFlow::Break(x)));
            }
            <Async as OneOf2>::arm_1(Async::ready(ControlFlow::Continue(())))
        });

        let r = block_on_test(f.async_());
        assert_eq!(r, 2)
    }

    #[cfg(any())]
    #[test]
    fn iter_next() {
        trait Iterator<C> {
            type Item;

            fn next(&mut self) -> impl Colored<C, Output = Option<Self::Item>>;
        }

        /*
            for x in colored_iter {}
        */

        fn run<C: Color>(i: impl Iterator<C, Item = ()>) -> impl Colored<C, Output = ()> {
            C::fold_mut(i, (), |x, k| {
                C::map(x.next(), |x1| match x1 {
                    Some(x1) => {
                        println!("{x1:#?}");
                        ControlFlow::Continue(())
                    }
                    None => ControlFlow::Break(()),
                })
            })
        }
    }
}
