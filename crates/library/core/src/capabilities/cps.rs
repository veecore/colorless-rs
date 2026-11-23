use crate::Colored;
use crate::capabilities::callbacks::{CallOnce, ColoredCallOnce};

/// # Continuation-Passing Style (CPS)
///
/// This trait provides the monadic foundations of the `colorless_core` framework.
///
/// In pure functional programming, monadic `bind` is an abstract equational operation.
/// In Rust, however, `Cps::and_then(first, next)` is a physical hardware contract governing
/// resource lifetimes and memory ordering.
///
/// ### The Drop-Order Invariant
///
/// In direct-style Rust, an `await` statement drops all intermediate temporaries before advancing:
///
/// ```rust,ignore
/// let result = step1().await;
/// // All RAII guards, lock handles, and buffers created by step1 are dropped right here.
/// step2(result).await;
/// ```
///
/// For a CPS implementation to behave correctly with Rust's ownership model, it must uphold this exact invariant:
/// **`first` and all of its temporaries must be completely evaluated and dropped before `next` is invoked.**
///
/// If an execution context retains `first` in memory while invoking `next`, any active lock guard
/// (such as a `MutexGuard`) or borrow held by `first` will overlap with `next`. In safe code, this produces
/// unexpected deadlocks (a step attempting to acquire a lock still held by its predecessor); in unsafe code,
/// it leads to aliased mutable references.
///
/// All unboxed state machines in Colorless guarantee in-place destruction of completed steps
/// before downstream continuations are evaluated.
///
/// ### The Call-Site Dual-Borrow Limitation
///
/// Because Rust evaluates function arguments eagerly at the call site, ordinary `Cps` cannot sequence
/// multiple mutable borrows of the same resource without assistance:
///
/// ```rust,ignore
/// // Rust rejects this at compile time:
/// C::and_then(compute(&mut x), move |_| compute(&mut x))
/// ```
///
/// This is not a deficiency of the execution model, but a consequence of Rust's call-site borrow checking.
/// For sequencing operations on shared mutable state, see the [`borrow`](crate::LinearBorrow1) capability module.
pub trait Cps: Linear + Sized {
    /// Maps a pure synchronous transformation over the output of a [`Colored`] computation.
    ///
    /// # Examples
    /// ```
    /// # use colorless_core::Color;
    ///
    /// # fn func<C: Color>() {
    ///  let doubled = C::map(C::ready(21), |x| x * 2);
    /// # }
    /// ```
    fn map<C, F>(c: C, f: F) -> impl Colored<Self, Output = F::Value>
    where
        C: Colored<Self>,
        F: CallOnce<C::Output>;

    /// Chains a continuation closure `next` to run after `first` resolves.
    ///
    /// This is the monadic flat-map / `bind` operation: `next` receives the output of `first`
    /// and returns a new [`Colored`] computation to be sequenced next.
    ///
    /// # Laziness and `wrap_flatten`
    ///
    /// `and_then` **sequences** steps but does not by itself defer when the chain is
    /// *constructed*. The laziness contract depends on the outer context:
    ///
    /// - In [`Identity`](crate::Identity): both `first` and the closure body execute
    ///   immediately and synchronously. There is no deferred scheduling.
    /// - In [`Async`](crate::Async): `and_then` wraps the sequence into an unboxed future.
    ///   Neither `first` nor `next` execute until that future is polled by the executor.
    fn and_then<C, F>(first: C, next: F) -> impl Colored<Self, Output = F::Value>
    where
        C: Colored<Self>,
        F: ColoredCallOnce<Self, C::Output>;
}

/// An execution context where monadic sequencing operations strictly maintain
/// non-overlapping, thread-synchronized linear execution.
///
/// # Safety
///
/// An implementor of `Linear` guarantees all five of the following invariants:
///
/// 1. **Strict Non-Overlapping Drop Order**:
///    In `C::and_then(first, next)`, `first` and all temporaries created during its
///    evaluation must be fully evaluated and dropped *before* `next` is invoked.
///    Retaining `first` while `next` runs causes destructor-induced aliasing and
///    conflicting mutations.
///
/// 2. **At-Most-Once Continuation Invocation**:
///    The continuation closure `next` is invoked at most once. It is never cloned,
///    re-executed, or shared across multiple evaluation paths. Multi-shot invocation
///    violates single-use affine states.
///
/// 3. **Inter-Thread Memory Synchronization**:
///    Consecutive steps must never run concurrently. If consecutive steps are executed
///    on different OS threads, the runtime must guarantee an Acquire-Release happens-before
///    barrier between the drop of `first` on the originating thread and the invocation of `next`
///    on the receiving thread. Failing to do so causes data races on unsynchronized pointers.
///
/// 4. **Synchronous Cancellation**:
///    If a computation is canceled/dropped while a step is in flight, the step must
///    be dropped synchronously. In-flight steps must never be detached to run in the
///    background. Detaching in-flight steps leads to Use-After-Free when the caller stack
///    reclaims the resource.
///
/// 5. **Poisoning on Panic**:
///    If any step panics or unwinds, `next` must not be invoked, and the engine
///    must never attempt to catch the unwind and retry or resume execution. Catching
///    and continuing invokes `next` on corrupted, half-mutated invariants .
pub unsafe trait Linear {}

#[cfg(all(test, feature = "rainbow"))]
mod pending_lock {
    use crate::Colored;
    use crate::colored::internal::{ColoredInternal, Present};
    use std::pin::Pin;
    use std::prelude::rust_2015::Box;
    use std::ptr::drop_in_place;
    use std::sync::{Arc, Mutex, MutexGuard};
    use std::task::{Context, Poll};

    pub struct LingeringLockStruct {
        mutex: Arc<Mutex<usize>>,
        guard: Option<MutexGuard<'static, usize>>,
        former_result: Option<usize>,
    }

    impl LingeringLockStruct {
        fn initiate(m: Arc<Mutex<usize>>) -> Pin<Box<Self>> {
            let this = Self {
                mutex: m,
                guard: None,
                former_result: None,
            };
            let mut l = Box::pin(this);
            let guard = l.mutex.lock().unwrap();
            // SAFETY: l is pinned
            let guard = unsafe { std::mem::transmute::<_, MutexGuard<'static, usize>>(guard) };
            l.former_result = Some(*guard);
            l.guard = Some(guard);
            l
        }
    }

    // a potentially lingering lock
    pub trait LingeringLock<C> {
        fn new(m: Arc<Mutex<usize>>) -> impl Colored<C, Output = usize>;
    }

    impl<C> LingeringLock<C> for LingeringLockStruct
    where
        Pin<Box<LingeringLockStruct>>: Colored<C, Output = usize>,
    {
        fn new(m: Arc<Mutex<usize>>) -> impl Colored<C, Output = usize> {
            Self::initiate(m)
        }
    }

    impl<C> Colored<C> for Pin<Box<LingeringLockStruct>> {
        type Output = usize;
    }

    // This general impl is bad for normal code but seems good for test... this is a good
    // hook point for all
    unsafe impl<C> ColoredInternal<C> for Pin<Box<LingeringLockStruct>> {
        type OutputInternal = usize;

        unsafe fn poll_colored(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<<C as Present>::Value<usize>>
        where
            C: Present,
        {
            Poll::Ready(C::from(self.former_result.unwrap()))
        }

        unsafe fn finalize_ready(mut self: Pin<&mut Self>) {
            let ptr: *mut Self = self.as_mut().get_mut();
            // if the combinators did not move the address
            unsafe { drop_in_place(ptr) }
        }

        fn compute_sync(self) -> <C as Present>::Value<Self::OutputInternal>
        where
            C: Present,
            Self: Sized,
        {
            C::from(self.former_result.unwrap())
        }
    }
}

#[cfg(all(test, feature = "rainbow"))]
mod builtin_color_test {
    use super::pending_lock::*;
    use crate::{
        AsyncExt, Color, Colored, FallibleAsyncExt, FallibleExt, FallibleOptionAsyncExt,
        FallibleOptionExt, IdentityExt, OneOf2, OptionalAsyncExt, OptionalExt,
        OptionalResultAsyncExt, OptionalResultExt, test_util::block_on_local_test,
    };
    use std::ops::ControlFlow;
    use std::sync::{Arc, Mutex};

    // For synchronous colors, the type-system enforces that there's only one
    // possible safe implementation - compute first argument, pass its output to the
    // next... perfectly linear - since the color can't safely manufacture Output
    // or Value out of thin air. Never returning doesn't count either.
    //
    // Only async can violate this since the colored-internal method for it takes
    // &mut which means it could mistakenly pend dropping the first colored.
    // The safety contracts for LinearSafe is exactly the-same required for safe
    // code so we test with lock which is easier to see fail
    #[test]
    fn cps_linear_test() {
        // Target code
        /*
            let val = lock(m).resolve;
            val + m.lock().expect("lock is released")
        */

        async fn native_async(mutex: Arc<Mutex<usize>>) -> usize {
            // The lock is dropped when the future is dropped
            let val = LingeringLockStruct::new(mutex.clone()).async_().await;

            val + *(mutex.lock().expect("lock is released"))
        }

        let r = block_on_local_test(native_async(Arc::new(Mutex::new(1))));
        assert_eq!(r, 2);

        fn func<C: Color, L>(mutex: Arc<Mutex<usize>>) -> impl Colored<C, Output = usize>
        where
            L: LingeringLock<C>,
        {
            C::wrap_flatten(move || {
                let val = L::new(mutex.clone());

                C::map(val, move |val| {
                    val + *(mutex.lock().expect("lock is released"))
                })
            })
        }

        fn func_and_then<C: Color, L>(mutex: Arc<Mutex<usize>>) -> impl Colored<C, Output = usize>
        where
            L: LingeringLock<C>,
        {
            C::wrap_flatten(move || {
                let val = L::new(mutex.clone());

                C::and_then(val, move |val| {
                    C::ready(val + *(mutex.lock().expect("lock is released")))
                })
            })
        }

        // async
        let r =
            block_on_local_test(func::<_, LingeringLockStruct>(Arc::new(Mutex::new(1))).async_());
        assert_eq!(r, 2);
        let r = block_on_local_test(
            func_and_then::<_, LingeringLockStruct>(Arc::new(Mutex::new(1))).async_(),
        );
        assert_eq!(r, 2);

        // async result
        let r = block_on_local_test(
            func::<_, LingeringLockStruct>(Arc::new(Mutex::new(1))).async_result_(),
        );
        assert_eq!(r, Ok::<_, ()>(2));
        let r = block_on_local_test(
            func_and_then::<_, LingeringLockStruct>(Arc::new(Mutex::new(1))).async_result_(),
        );
        assert_eq!(r, Ok::<_, ()>(2));

        // async result option
        let r = block_on_local_test(
            func::<_, LingeringLockStruct>(Arc::new(Mutex::new(1))).async_result_option_(),
        );
        assert_eq!(r, Ok::<_, ()>(Some(2)));
        let r = block_on_local_test(
            func_and_then::<_, LingeringLockStruct>(Arc::new(Mutex::new(1))).async_result_option_(),
        );
        assert_eq!(r, Ok::<_, ()>(Some(2)));

        // async option
        let r = block_on_local_test(
            func::<_, LingeringLockStruct>(Arc::new(Mutex::new(1))).async_option_(),
        );
        assert_eq!(r, Some(2));
        let r = block_on_local_test(
            func_and_then::<_, LingeringLockStruct>(Arc::new(Mutex::new(1))).async_option_(),
        );
        assert_eq!(r, Some(2));

        // async option result
        let r = block_on_local_test(
            func::<_, LingeringLockStruct>(Arc::new(Mutex::new(1))).async_option_result_(),
        );
        assert_eq!(r, Some(Ok::<_, ()>(2)));
        let r = block_on_local_test(
            func_and_then::<_, LingeringLockStruct>(Arc::new(Mutex::new(1))).async_option_result_(),
        );
        assert_eq!(r, Some(Ok::<_, ()>(2)));

        // sync
        let r = func::<_, LingeringLockStruct>(Arc::new(Mutex::new(1))).sync_();
        assert_eq!(r, 2);
        let r = func_and_then::<_, LingeringLockStruct>(Arc::new(Mutex::new(1))).sync_();
        assert_eq!(r, 2);

        // result
        let r = func::<_, LingeringLockStruct>(Arc::new(Mutex::new(1))).result_();
        assert_eq!(r, Ok::<_, ()>(2));
        let r = func_and_then::<_, LingeringLockStruct>(Arc::new(Mutex::new(1))).result_();
        assert_eq!(r, Ok::<_, ()>(2));

        // result option
        let r = func::<_, LingeringLockStruct>(Arc::new(Mutex::new(1))).result_option_();
        assert_eq!(r, Ok::<_, ()>(Some(2)));
        let r = func_and_then::<_, LingeringLockStruct>(Arc::new(Mutex::new(1))).result_option_();
        assert_eq!(r, Ok::<_, ()>(Some(2)));

        // option
        let r = func::<_, LingeringLockStruct>(Arc::new(Mutex::new(1))).option_();
        assert_eq!(r, Some(2));
        let r = func_and_then::<_, LingeringLockStruct>(Arc::new(Mutex::new(1))).option_();
        assert_eq!(r, Some(2));

        // option result
        let r = func::<_, LingeringLockStruct>(Arc::new(Mutex::new(1))).option_result_();
        assert_eq!(r, Some(Ok::<_, ()>(2)));
        let r = func_and_then::<_, LingeringLockStruct>(Arc::new(Mutex::new(1))).option_result_();
        assert_eq!(r, Some(Ok::<_, ()>(2)));
    }

    // same goal... we test that the former computation is dropped
    #[test]
    fn loop_linear_test() {
        // Target code
        /*
            loop {
                // This one line is enough to test since it'd
                // try to lock again on initiate but that test
                // is not ours
                lock(m).resolve;
                // so we enforce it
                m.lock().expect("lock is released");
            }
        */

        fn func<C: Color, L>(mutex: Arc<Mutex<usize>>) -> impl Colored<C, Output = usize>
        where
            L: LingeringLock<C>,
        {
            let counts = 0;
            C::fold((counts, mutex), move |(mut counts, mutex)| {
                if counts > 0 {
                    <C as OneOf2>::arm_0(C::ready(ControlFlow::Break(1 + *(mutex.lock().unwrap()))))
                } else {
                    counts += 1;
                    let else_ = C::wrap_flatten(move || {
                        let val = L::new(mutex.clone());

                        C::map(val, move |_val| {
                            drop(mutex.lock().expect("lock is released"));
                            ControlFlow::Continue((counts, mutex))
                        })
                    });
                    <C as OneOf2>::arm_1(else_)
                }
            })
        }

        // async
        let r =
            block_on_local_test(func::<_, LingeringLockStruct>(Arc::new(Mutex::new(1))).async_());
        assert_eq!(r, 2);

        // async result
        let r = block_on_local_test(
            func::<_, LingeringLockStruct>(Arc::new(Mutex::new(1))).async_result_(),
        );
        assert_eq!(r, Ok::<_, ()>(2));

        // async result option
        let r = block_on_local_test(
            func::<_, LingeringLockStruct>(Arc::new(Mutex::new(1))).async_result_option_(),
        );
        assert_eq!(r, Ok::<_, ()>(Some(2)));

        // async option
        let r = block_on_local_test(
            func::<_, LingeringLockStruct>(Arc::new(Mutex::new(1))).async_option_(),
        );
        assert_eq!(r, Some(2));

        // async option result
        let r = block_on_local_test(
            func::<_, LingeringLockStruct>(Arc::new(Mutex::new(1))).async_option_result_(),
        );
        assert_eq!(r, Some(Ok::<_, ()>(2)));

        // sync
        let r = func::<_, LingeringLockStruct>(Arc::new(Mutex::new(1))).sync_();
        assert_eq!(r, 2);

        // result
        let r = func::<_, LingeringLockStruct>(Arc::new(Mutex::new(1))).result_();
        assert_eq!(r, Ok::<_, ()>(2));

        // result option
        let r = func::<_, LingeringLockStruct>(Arc::new(Mutex::new(1))).result_option_();
        assert_eq!(r, Ok::<_, ()>(Some(2)));

        // option
        let r = func::<_, LingeringLockStruct>(Arc::new(Mutex::new(1))).option_();
        assert_eq!(r, Some(2));

        // option result
        let r = func::<_, LingeringLockStruct>(Arc::new(Mutex::new(1))).option_result_();
        assert_eq!(r, Some(Ok::<_, ()>(2)));
    }
}
