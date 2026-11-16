/// # The Asynchronous Execution Context
///
/// The [`Async`] color lowers abstract `Colored<Async>` computations into native, unboxed Rust futures
/// driven directly by standard asynchronous executors (e.g. Tokio).
///
/// ### Why Unboxed Enums?
///
/// An easy way to implement monadic async combinators is to wrap each stage in a boxed future
/// (`Pin<Box<dyn Future>>`) or a nested `async move { ... }` block.
///
/// While simple, nesting `async` blocks causes a state machine's memory layout to approximately
/// double at every composition step: an outer future stores the inner future, which stores its predecessor,
/// resulting in combinatorial memory explosion.
///
/// `colorless` uses explicit, compact unboxed state machines.
/// Each state machine is an enum that stores only the active flow, switching in-place to the next stage
/// upon completion.
///
/// ### In-Place Destruction and Self-Referential Pinning
///
/// A central challenge in writing unboxed future state machines is dropping a completed stage
/// without moving it across memory.
///
/// Under the contract of [`core::pin::Pin`]), a `!Unpin` future must never be moved in memory from the moment it is
/// pinned until its `Drop` implementation has finished executing. If a state machine uses `mem::replace`
/// to take a completed future out of its pin before dropping it, the future is moved to the caller's stack frame.
/// If that future relies on self-referential pointers in its destructor, moving it causes immediate memory corruption.
///
/// Colorless resolves this by wrapping in-flight flows in [`core::mem::ManuallyDrop`]. When a step completes,
/// its destructor is invoked in-place at its pinned address via `ManuallyDrop::drop(&mut this.flow)`.
/// The memory address never changes, the lock/borrow is released immediately, and the compiler is prevented
/// from generating a second, duplicate drop call when the outer future is deallocated.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Async;

pub use impls::AsyncExt;

mod impls {
    use crate::Async;

    macro_rules! native_exec {
        ($c:expr => $v:ident $ok:expr) => {
            match $c {
                $v => $ok,
            }
        };
        (@map $c:expr => $v:ident $ok:expr) => {
            match $c {
                $v => $ok,
            }
        };
        (@loop $c:expr => $v:ident $ok:expr) => {
            match $c {
                ControlFlow::Break(b) => return b,
                ControlFlow::Continue($v) => $ok,
            }
        };
        (@init $expr:expr) => {
            $expr
        };
    }

    macro_rules! poll_exec {
        (@init $value:expr) => {
            $value
        };
        (
            $t:expr;

            on_pending: $p:block

            on_ready: $r:block

            $output:ident on_ready_ok: $ok:block
        ) => {
            match $t {
                Poll::Pending => {
                    $p
                },
                Poll::Ready($output) => {
                    $ok
                }
            }
        };
        (
            @loop

            $t:expr;

            on_pending: $p:block

            on_ready: $r:block

            $output:ident on_ready_ok: $ok:block
        ) => {
            match $t {
                Poll::Pending => {
                    $p
                },
                Poll::Ready(ControlFlow::Break(value)) => {
                    $r;
                    return Poll::Ready(value);
                },
                Poll::Ready(ControlFlow::Continue($output)) => {
                    $ok
                }
            }
        }
    }

    declare_async_all! {
        AsyncExt,
        <> Async
        Self::Output,
        T,
        async_,
        native_exec,
        poll_exec,
    }
}

#[cfg(test)]
mod parity_test {
    use super::*;
    use crate::colors::async_::identity::impls::{AsyncExt, MakeColored};
    use crate::{Cps, Loop, OneOf2, Ready, Wrap};
    use std::future::poll_fn;
    use std::ops::ControlFlow;
    use std::task::Poll;

    fn layout_countdown(value: i64) -> impl Future<Output = i64> {
        let mut remaining = 1_i64;
        poll_fn(move |cx| {
            if remaining == 0 {
                Poll::Ready(value)
            } else {
                remaining -= 1;
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        })
    }

    // TODO: the current implementations that use async block need to be
    //  updated soon
    #[test]
    fn manual_combinator_layouts_do_not_exceed_native_futures() {
        #[derive(Clone, Copy)]
        struct LayoutPayload([usize; 3]);

        impl LayoutPayload {
            #[inline(always)]
            fn total(self) -> usize {
                self.0.into_iter().sum()
            }
        }

        fn assert_compact_layout(operation: &str, colored: &impl Future, native: &impl Future) {
            let colored_size = size_of_val(colored);
            let native_size = size_of_val(native);
            assert!(
                colored_size <= native_size,
                "{operation}: Colorless future is {colored_size} bytes, larger than the {native_size}-byte native future"
            );
            let colored_align = align_of_val(colored);
            let native_align = align_of_val(native);
            assert_eq!(
                colored_align, native_align,
                "{operation}: Colorless future has alignment of {colored_align}, native future is {native_align}"
            );
        }

        fn native_ready(value: LayoutPayload) -> impl Future<Output = LayoutPayload> {
            core::future::ready(value)
        }

        fn colored_ready(value: LayoutPayload) -> impl Future<Output = LayoutPayload> {
            Async::ready(value).async_()
        }

        fn native_wrap(value: LayoutPayload) -> impl Future<Output = usize> {
            async move { value.total() }
        }

        fn colored_wrap(value: LayoutPayload) -> impl Future<Output = usize> {
            Async::wrap(move || value.total()).async_()
        }

        fn native_flatten(value: i64) -> impl Future<Output = i64> {
            async move { layout_countdown(value).await }
        }

        fn colored_flatten(value: i64) -> impl Future<Output = i64> {
            Async::wrap_flatten(move || {
                Async::map(MakeColored(layout_countdown(value)), |value| value)
            })
                .async_()
        }

        fn native_map(value: i64, captured: LayoutPayload) -> impl Future<Output = usize> {
            async move { layout_countdown(value).await as usize + captured.total() }
        }

        fn colored_map(value: i64, captured: LayoutPayload) -> impl Future<Output = usize> {
            Async::wrap_flatten(move || {
                Async::map(MakeColored(layout_countdown(value)), move |value| {
                    value as usize + captured.total()
                })
            })
                .async_()
        }

        fn native_and_then(value: i64, captured: LayoutPayload) -> impl Future<Output = i64> {
            async move {
                let value = layout_countdown(value).await;
                layout_countdown(value + captured.total() as i64).await
            }
        }

        fn colored_and_then(value: i64, captured: LayoutPayload) -> impl Future<Output = i64> {
            Async::wrap_flatten(move || {
                Async::and_then(MakeColored(layout_countdown(value)), move |value| {
                    MakeColored(layout_countdown(value + captured.total() as i64))
                })
            })
                .async_()
        }

        fn native_loop(value: i64, captured: LayoutPayload) -> impl Future<Output = i64> {
            async move {
                loop {
                    let r = layout_countdown(value).await;
                    if r == value {
                        return value + captured.total() as i64;
                    }
                }
            }
        }

        fn colored_loop(value: i64, captured: LayoutPayload) -> impl Future<Output = i64> {
            Async::wrap_flatten(move || {
                Async::fold((captured, value), move |(capture, value)| {
                    Async::map(MakeColored(layout_countdown(value)), move |r| {
                        return if r == value {
                            ControlFlow::Break(value + capture.total() as i64)
                        } else {
                            ControlFlow::Continue((capture, value))
                        };
                    })
                })
            })
                .async_()
        }

        let payload = LayoutPayload([1, 2, 3]);

        assert_compact_layout("ready", &colored_ready(payload), &native_ready(payload));
        assert_compact_layout("wrap", &colored_wrap(payload), &native_wrap(payload));
        assert_compact_layout("wrap_flatten", &colored_flatten(1), &native_flatten(1));
        assert_compact_layout("map", &colored_map(1, payload), &native_map(1, payload));
        assert_compact_layout(
            "and_then",
            &colored_and_then(1, payload),
            &native_and_then(1, payload),
        );

        assert_compact_layout("loop", &colored_loop(1, payload), &native_loop(1, payload));
    }

    #[test]
    fn fold_layout_does_not_exceed_native_async() {
        async fn native(n: i64) -> i64 {
            let mut sum = 0;
            let mut i = 0;
            while i < n {
                sum += layout_countdown(i).await;
                i += 1;
            }
            sum
        }

        fn colored(n: i64) -> impl Future<Output = i64> {
            Async::wrap_flatten(move || {
                Async::fold((n, 0_i64, 0_i64), move |(n, mut sum, mut i)| {
                    if i < n {
                        <Async as OneOf2>::arm_0(Async::map(
                            MakeColored(layout_countdown(i)),
                            move |value| {
                                sum += value;
                                i += 1;
                                ControlFlow::Continue((n, sum, i))
                            },
                        ))
                    } else {
                        <Async as OneOf2>::arm_1(Async::ready(ControlFlow::Break(sum)))
                    }
                })
            })
                .async_()
        }

        let native = native(10);
        let colored = colored(10);
        assert!(
            size_of_val(&colored) <= size_of_val(&native),
            "fold must not exceed the native async state-machine size"
        );
        #[cfg(target_pointer_width = "64")]
        assert_eq!(
            size_of_val(&colored),
            48,
            "fold future layout regressed on a 64-bit target"
        );
    }
}

#[cfg(test)]
mod drop_tests {
    use super::*;
    use crate::callbacks::{CallOnce, ColoredCallOnce, ColoredThunkOnce, ThunkOnce};
    use crate::colors::async_::identity::impls::{AsyncExt, MakeColored};
    use crate::{test_util::{block_on_local_test, block_on_test, fetch, poll_once}, Colored, Colorize, Cps, Loop, Ready, Wrap};
    use std::future::poll_fn;
    use std::hint::black_box;
    use std::ops::ControlFlow;
    use std::panic::AssertUnwindSafe;
    use std::pin::{pin, Pin};
    use std::prelude::v1::Box;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::task::{Context, Poll};

    #[test]
    fn unpolled_drops_capture() {
        let capture_drops = test_spy();
        let future = Async::wrap(drop_spy_body_panic_on_call(capture_drops.clone())).async_();
        drop(future);
        assert_eq!(capture_drops.load(Ordering::SeqCst), 1);

        // test that it drops even as Colored<Async>
        let capture_drops = test_spy();
        let future = Async::wrap(drop_spy_body_panic_on_call(capture_drops.clone()));
        drop(future);
        assert_eq!(capture_drops.load(Ordering::SeqCst), 1);

        // test flatten
        let capture_drops = test_spy();
        let future =
            Async::wrap_flatten(drop_spy_body_panic_on_call(capture_drops.clone())).async_();
        drop(future);
        assert_eq!(capture_drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn make_future_transitions_from_pending_to_terminated_exactly_once() {
        struct PendingThenReady {
            pending: bool,
            _capture: DropProbe,
        }

        impl Future for PendingThenReady {
            type Output = usize;

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                if self.pending {
                    self.pending = false;
                    cx.waker().wake_by_ref();
                    Poll::Pending
                } else {
                    Poll::Ready(7)
                }
            }
        }

        let drops = test_spy();
        let future = MakeColored(PendingThenReady {
            pending: true,
            _capture: DropProbe(drops.clone()),
        })
            .async_();
        let mut future = pin!(future);

        assert!(poll_once(future.as_mut()).is_pending());
        assert_eq!(drops.load(Ordering::SeqCst), 0);

        assert_eq!(poll_once(future.as_mut()), Poll::Ready(7));
        assert_eq!(drops.load(Ordering::SeqCst), 1);

        assert!(
            std::panic::catch_unwind(AssertUnwindSafe(|| poll_once(future.as_mut()))).is_err(),
            "a terminated MakeFuture must reject another poll"
        );
        drop(future);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn make_future_is_terminated_when_ready_finalization_panics() {
        let drops = test_spy();
        let future = ready_computation_panic_on_drop(drops.clone()).async_();
        let mut future = pin!(future);

        assert!(
            std::panic::catch_unwind(AssertUnwindSafe(|| poll_once(future.as_mut()))).is_err(),
            "ready finalization must propagate its destructor panic"
        );
        assert_eq!(drops.load(Ordering::SeqCst), 1);

        assert!(
            std::panic::catch_unwind(AssertUnwindSafe(|| poll_once(future.as_mut()))).is_err(),
            "a MakeFuture whose ready finalization panicked must stay terminated"
        );
        drop(future);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn continuation_drops_in_flight() {
        // this isn't a parity test... 
        // native async doesn't do this, but it's just inevitable with the current design
        let thunk_drops = test_spy();
        let flow_drops = test_spy();
        let future = Async::wrap_flatten({
            let capture = DropProbe(thunk_drops.clone());
            let flow_drops = flow_drops.clone();
            move || {
                use_(&capture);
                pending_computation_drop_spy(flow_drops)
            }
        })
            .async_();

        let mut future = Box::pin(future);
        assert!(poll_once(future.as_mut()).is_pending());
        assert_eq!(thunk_drops.load(Ordering::SeqCst), 1);
        // the computation produced by the closure is still active even though the
        // closure had been dropped
        assert_eq!(flow_drops.load(Ordering::SeqCst), 0);
        drop(future);
        assert_eq!(flow_drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn head_drops_in_flight() {
        fn assert_func(
            source_drops: Arc<AtomicUsize>,
            continuation_drops: Arc<AtomicUsize>,
            future: impl Future,
        ) {
            let mut future = Box::pin(future);
            assert!(poll_once(future.as_mut()).is_pending());
            drop(future);
            assert_eq!(source_drops.load(Ordering::SeqCst), 1);
            assert_eq!(continuation_drops.load(Ordering::SeqCst), 1);
        }

        let source_drops = test_spy();
        let continuation_drops = test_spy();
        let future = Async::map(
            pending_computation_drop_spy(source_drops.clone()),
            drop_spy_continuation_panic_on_call(continuation_drops.clone()),
        )
            .async_();
        assert_func(source_drops, continuation_drops, future);

        let source_drops = test_spy();
        let continuation_drops = test_spy();
        let future = Async::and_then(
            pending_computation_drop_spy(source_drops.clone()),
            drop_spy_continuation_panic_on_call(continuation_drops.clone()),
        )
            .async_();
        assert_func(source_drops, continuation_drops, future);
    }

    #[test]
    fn head_and_tail_drops_flight() {
        let first_drops = test_spy();
        let then_drops = test_spy();
        let future = Async::and_then(ready_computation_drop_spy(first_drops.clone()), |()| {
            pending_computation_drop_spy(then_drops.clone())
        })
            .async_();

        let mut future = Box::pin(future);
        // on ready, it goes to pending immediately
        assert!(poll_once(future.as_mut()).is_pending());
        assert_eq!(first_drops.load(Ordering::SeqCst), 1);
        assert_eq!(then_drops.load(Ordering::SeqCst), 0);
        drop(future);
        assert_eq!(then_drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn continuation_is_not_leaked_on_source_panic_on_drop() {
        fn assert_func(
            source_drops: Arc<AtomicUsize>,
            continuation_drops: Arc<AtomicUsize>,
            future: impl Future,
        ) {
            assert!(
                std::panic::catch_unwind(AssertUnwindSafe(|| drop(future))).is_err(),
                "cancelling map must propagate the source destructor panic"
            );
            assert_eq!(source_drops.load(Ordering::SeqCst), 1);
            assert_eq!(continuation_drops.load(Ordering::SeqCst), 1);
        }

        let source_drops = test_spy();
        let continuation_drops = test_spy();
        let future = Async::map(
            ready_computation_panic_on_drop(source_drops.clone()),
            drop_spy_continuation_panic_on_call(continuation_drops.clone()),
        )
            .async_();
        assert_func(source_drops, continuation_drops, future);

        let source_drops = test_spy();
        let continuation_drops = test_spy();
        let future = Async::and_then(
            ready_computation_panic_on_drop(source_drops.clone()),
            drop_spy_continuation_panic_on_call(continuation_drops.clone()),
        )
            .async_();
        assert_func(source_drops, continuation_drops, future);
    }

    #[test]
    fn continuation_is_not_leaked_on_source_panic_in_flight() {
        fn assert_func(
            source_drops: Arc<AtomicUsize>,
            continuation_drops: Arc<AtomicUsize>,
            future: impl Future,
        ) {
            let mut future = Box::pin(future);
            assert!(
                std::panic::catch_unwind(AssertUnwindSafe(|| poll_once(future.as_mut()))).is_err()
            );
            drop(future);
            assert_eq!(source_drops.load(Ordering::SeqCst), 1);
            assert_eq!(continuation_drops.load(Ordering::SeqCst), 1);
        }

        let source_drops = test_spy();
        let continuation_drops = test_spy();
        let future = Async::map(
            ready_computation_panic_on_drop(source_drops.clone()),
            drop_spy_continuation_panic_on_call(continuation_drops.clone()),
        )
            .async_();
        assert_func(source_drops, continuation_drops, future);

        let source_drops = test_spy();
        let continuation_drops = test_spy();
        let future = Async::and_then(
            ready_computation_panic_on_drop(source_drops.clone()),
            drop_spy_continuation_panic_on_call(continuation_drops.clone()),
        )
            .async_();
        assert_func(source_drops, continuation_drops, future);
    }

    #[test]
    fn panicking_body_and_continuation_drop_captures_and_poison_state() {
        fn assert_func(capture_drops: Arc<AtomicUsize>, future: impl Future) {
            let mut future = Box::pin(future);
            assert!(
                std::panic::catch_unwind(AssertUnwindSafe(|| poll_once(future.as_mut()))).is_err()
            );
            assert_eq!(capture_drops.load(Ordering::SeqCst), 1);
            // must not allow repoll
            assert!(
                std::panic::catch_unwind(AssertUnwindSafe(|| poll_once(future.as_mut()))).is_err()
            );
            drop(future);
            // Must only be dropped once
            assert_eq!(capture_drops.load(Ordering::SeqCst), 1);
        }

        // wrap
        let capture_drops = test_spy();
        let future = Async::wrap(drop_spy_body_panic_on_call(capture_drops.clone())).async_();
        assert_func(capture_drops, future);

        // wrap_flatten
        let capture_drops = test_spy();
        let future =
            Async::wrap_flatten(drop_spy_body_panic_on_call(capture_drops.clone())).async_();
        assert_func(capture_drops, future);

        // wrap_flatten
        let capture_drops = test_spy();
        let future =
            Async::wrap_flatten(drop_spy_body_panic_on_call(capture_drops.clone())).async_();
        assert_func(capture_drops, future);

        // map
        let source_drops = test_spy();
        let capture_drops = test_spy();
        let future = Async::map(
            ready_computation_drop_spy(source_drops.clone()),
            drop_spy_continuation_panic_on_call(capture_drops.clone()),
        )
            .async_();
        assert_func(capture_drops, future);
        assert_eq!(source_drops.load(Ordering::SeqCst), 1);

        // and_then
        let source_drops = test_spy();
        let capture_drops = test_spy();
        let future = Async::and_then(
            ready_computation_drop_spy(source_drops.clone()),
            drop_spy_continuation_panic_on_call(capture_drops.clone()),
        )
            .async_();
        assert_func(capture_drops, future);
        assert_eq!(source_drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn fold_drops_only_the_active_phase() {
        struct PendingLoopFlow {
            _state: Option<DropProbe>,
            drops: Arc<AtomicUsize>,
        }

        impl Future for PendingLoopFlow {
            type Output = ControlFlow<(), DropProbe>;

            fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
                Poll::Pending
            }
        }

        impl Drop for PendingLoopFlow {
            fn drop(&mut self) {
                self.drops.fetch_add(1, Ordering::SeqCst);
            }
        }

        fn generate_future(
            state_drops: Arc<AtomicUsize>,
            flow_drops: Arc<AtomicUsize>,
            callback_drops: Arc<AtomicUsize>,
        ) -> impl Future {
            let callback_capture = DropProbe(callback_drops);
            Async::fold(DropProbe(state_drops), move |state| {
                use_(&callback_capture);
                MakeColored(PendingLoopFlow {
                    _state: Some(state),
                    drops: flow_drops.clone(),
                })
            })
                .async_()
        }

        let state_drops = test_spy();
        let flow_drops = test_spy();
        let callback_drops = test_spy();
        let unpolled = generate_future(
            state_drops.clone(),
            flow_drops.clone(),
            callback_drops.clone(),
        );
        drop(unpolled);
        assert_eq!(state_drops.load(Ordering::SeqCst), 1);
        assert_eq!(flow_drops.load(Ordering::SeqCst), 0);
        assert_eq!(callback_drops.load(Ordering::SeqCst), 1);

        let state_drops = test_spy();
        let flow_drops = test_spy();
        let callback_drops = test_spy();
        let running = generate_future(
            state_drops.clone(),
            flow_drops.clone(),
            callback_drops.clone(),
        );
        let mut running = Box::pin(running);
        assert!(poll_once(running.as_mut()).is_pending());
        assert_eq!(callback_drops.load(Ordering::SeqCst), 0);
        drop(running);

        assert_eq!(state_drops.load(Ordering::SeqCst), 1);
        assert_eq!(flow_drops.load(Ordering::SeqCst), 1);
        assert_eq!(callback_drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn fold_is_poisoned_when_the_running_flow_drop_panics() {
        struct PanicOnDrop {
            value: Option<DropProbe>,
            drops: Arc<AtomicUsize>,
        }

        impl Future for PanicOnDrop {
            type Output = ControlFlow<DropProbe, ()>;

            fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
                Poll::Ready(ControlFlow::Break(
                    self.value.take().expect("flow polled after completion"),
                ))
            }
        }

        impl Drop for PanicOnDrop {
            fn drop(&mut self) {
                self.drops.fetch_add(1, Ordering::SeqCst);
                panic!("loop flow drop panic");
            }
        }

        let callback_drops = test_spy();
        let flow_drops = test_spy();
        let value_drops = test_spy();
        let callback_capture = DropProbe(callback_drops.clone());
        let flow_drops_in_callback = flow_drops.clone();
        let value_drops_in_callback = value_drops.clone();
        let future = Async::fold((), move |_| {
            use_(&callback_capture);
            MakeColored(PanicOnDrop {
                value: Some(DropProbe(value_drops_in_callback.clone())),
                drops: flow_drops_in_callback.clone(),
            })
        })
            .async_();
        let mut future = pin!(future);

        let panic = std::panic::catch_unwind(AssertUnwindSafe(|| block_on_test(&mut future)))
            .expect_err("the running flow destructor must run before fold completes");
        let message: Box<&str> = panic.downcast().unwrap();
        assert_eq!(*message, "loop flow drop panic");
        assert_eq!(flow_drops.load(Ordering::SeqCst), 1);
        assert_eq!(value_drops.load(Ordering::SeqCst), 1);
        assert_eq!(callback_drops.load(Ordering::SeqCst), 1);

        let panic = std::panic::catch_unwind(AssertUnwindSafe(|| block_on_local_test(future)))
            .expect_err("a fold whose active flow panicked in Drop must stay poisoned");
        let message: Box<&str> = panic.downcast().unwrap();
        assert!(message.contains("panic"));
        assert_eq!(flow_drops.load(Ordering::SeqCst), 1);
        assert_eq!(value_drops.load(Ordering::SeqCst), 1);
        assert_eq!(callback_drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn fold_drops_continue_state_when_the_running_flow_drop_panics() {
        struct ContinueThenPanicOnDrop {
            state: Option<DropProbe>,
            drops: Arc<AtomicUsize>,
        }

        impl Future for ContinueThenPanicOnDrop {
            type Output = ControlFlow<(), DropProbe>;

            fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
                Poll::Ready(ControlFlow::Continue(
                    self.state.take().expect("flow polled after completion"),
                ))
            }
        }

        impl Drop for ContinueThenPanicOnDrop {
            fn drop(&mut self) {
                self.drops.fetch_add(1, Ordering::SeqCst);
                panic!("loop flow drop panic");
            }
        }

        let state_drops = test_spy();
        let callback_drops = test_spy();
        let flow_drops = test_spy();
        let callback_capture = DropProbe(callback_drops.clone());
        let flow_drops_in_callback = flow_drops.clone();
        let future = Async::fold(DropProbe(state_drops.clone()), move |state| {
            use_(&callback_capture);
            MakeColored(ContinueThenPanicOnDrop {
                state: Some(state),
                drops: flow_drops_in_callback.clone(),
            })
        })
            .async_();
        let mut future = pin!(future);

        let panic = std::panic::catch_unwind(AssertUnwindSafe(|| block_on_test(&mut future)))
            .expect_err("the running flow destructor must run before the next iteration");
        let message: Box<&str> = panic.downcast().unwrap();
        assert_eq!(*message, "loop flow drop panic");
        assert_eq!(flow_drops.load(Ordering::SeqCst), 1);
        assert_eq!(state_drops.load(Ordering::SeqCst), 1);
        assert_eq!(callback_drops.load(Ordering::SeqCst), 1);

        assert!(
            std::panic::catch_unwind(AssertUnwindSafe(|| block_on_local_test(future))).is_err(),
            "a fold whose active flow panicked in Drop must stay poisoned"
        );
        assert_eq!(flow_drops.load(Ordering::SeqCst), 1);
        assert_eq!(state_drops.load(Ordering::SeqCst), 1);
        assert_eq!(callback_drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn fold_drops_callback_on_successful_break() {
        let callback_drops = test_spy();
        let callback_capture = DropProbe(callback_drops.clone());
        let future = Async::fold((), move |_| {
            use_(&callback_capture);
            Async::ready(ControlFlow::<usize, ()>::Break(7))
        })
            .async_();

        assert_eq!(block_on_test(future), 7);
        assert_eq!(callback_drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn fold_drops_each_completed_flow_before_calling_the_next_step() {
        struct ReadyLoopFlow {
            output: Option<ControlFlow<usize, usize>>,
            live: Arc<AtomicUsize>,
        }

        impl Future for ReadyLoopFlow {
            type Output = ControlFlow<usize, usize>;

            fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
                Poll::Ready(self.output.take().expect("flow polled after completion"))
            }
        }

        impl Drop for ReadyLoopFlow {
            fn drop(&mut self) {
                assert_eq!(
                    self.live.fetch_sub(1, Ordering::SeqCst),
                    1,
                    "more than one iteration flow was alive"
                );
            }
        }

        let live = test_spy();
        let calls = test_spy();
        let live_in_callback = live.clone();
        let calls_in_callback = calls.clone();
        let future = Async::fold(0, move |state| {
            assert_eq!(
                live_in_callback.fetch_add(1, Ordering::SeqCst),
                0,
                "the previous iteration flow must be dropped before the callback is called"
            );
            calls_in_callback.fetch_add(1, Ordering::SeqCst);

            let output = if state == 0 {
                ControlFlow::Continue(1)
            } else {
                ControlFlow::Break(7)
            };
            MakeColored(ReadyLoopFlow {
                output: Some(output),
                live: live_in_callback.clone(),
            })
        })
            .async_();

        assert_eq!(block_on_test(future), 7);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(live.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn fold_is_poisoned_when_the_running_flow_poll_panics() {
        struct PanicOnPoll(Arc<AtomicUsize>);

        impl Future for PanicOnPoll {
            type Output = ControlFlow<(), ()>;

            fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
                panic!("loop flow poll panic");
            }
        }

        impl Drop for PanicOnPoll {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }

        let drops = Arc::new(AtomicUsize::new(0));
        let callback_drops = test_spy();
        let callback_capture = DropProbe(callback_drops.clone());
        let future = Async::fold((), {
            let drops = drops.clone();
            move |_| {
                use_(&callback_capture);
                MakeColored(PanicOnPoll(drops.clone()))
            }
        })
            .async_();

        let mut future = pin!(future);

        let panic = std::panic::catch_unwind(AssertUnwindSafe(|| block_on_test(&mut future)))
            .expect_err("the running flow must propagate its poll panic");
        let message: Box<&str> = panic.downcast().unwrap();
        assert_eq!(*message, "loop flow poll panic");
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        assert_eq!(callback_drops.load(Ordering::SeqCst), 1);

        let panic = std::panic::catch_unwind(AssertUnwindSafe(|| block_on_test(future)))
            .expect_err("a fold whose active flow panicked in poll must stay poisoned");
        let message: Box<&str> = panic.downcast().unwrap();
        assert!(message.contains("panic"));
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        assert_eq!(callback_drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn fold_drops_state_and_callback_when_the_callback_panics() {
        let state_drops = test_spy();
        let callback_drops = test_spy();
        let callback_capture = DropProbe(callback_drops.clone());

        let future = Async::fold(DropProbe(state_drops.clone()), move |state| {
            use_(&callback_capture);
            use_(&state);
            panic!("loop callback panic");

            #[allow(unreachable_code)]
            Async::ready(ControlFlow::<(), DropProbe>::Break(()))
        })
            .async_();
        let mut future = pin!(future);

        let panic = std::panic::catch_unwind(AssertUnwindSafe(|| block_on_test(&mut future)))
            .expect_err("the loop callback panic must propagate");
        let message: Box<&str> = panic.downcast().unwrap();
        assert_eq!(*message, "loop callback panic");
        assert_eq!(state_drops.load(Ordering::SeqCst), 1);
        assert_eq!(callback_drops.load(Ordering::SeqCst), 1);

        assert!(
            std::panic::catch_unwind(AssertUnwindSafe(|| block_on_local_test(future))).is_err(),
            "a fold whose callback panicked must stay poisoned"
        );
        assert_eq!(state_drops.load(Ordering::SeqCst), 1);
        assert_eq!(callback_drops.load(Ordering::SeqCst), 1);
    }

    // TODO: ADD AS PROPER TEST... I JUST USED IT AS VERIFICATION FOR LOOP
    #[test]
    fn future_dropped_after_poll_panic_on_unwind() {
        fn panic_on_poll(drops: Arc<AtomicUsize>) -> impl Future<Output = ()> {
            let d = DropProbe(drops);
            poll_fn(move |_| {
                use_(&d);
                panic!("poll panic")
            })
        }

        let drops = test_spy();
        // let future = async {
        //     panic_on_poll(drops.clone()).await;
        // };

        let future = Async::wrap_flatten(|| MakeColored(panic_on_poll(drops.clone()))).async_();

        {
            let mut future = pin!(future);
            let panic = std::panic::catch_unwind(AssertUnwindSafe(|| block_on_test(&mut future)))
                .expect_err("the running flow must propagate its poll panic");
            let message: Box<&str> = panic.downcast().unwrap();
            assert_eq!(*message, "poll panic");
            assert_eq!(drops.load(Ordering::SeqCst), 1);

            let panic = std::panic::catch_unwind(AssertUnwindSafe(|| block_on_test(future)))
                .expect_err("a fold whose active flow panicked in poll must stay poisoned");
            let message: Box<&str> = panic.downcast().unwrap();
            assert!(message.contains("panic"));
        }
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    // TODO: this is a limitation... this test will fail... expressed
    //  already in continuation_drops_in_flight
    #[test]
    #[should_panic(expected = "left: 1")]
    fn continuation_stack_vars_are_held_till_completion() {
        /*
            fetch().resolve;
            let r = resource();
            fetch().resolve;
            assert!(r_not_dropped);
        */
        let drops = Arc::new(AtomicUsize::new(0));
        let future = Async::wrap_flatten(move || {
            Async::and_then(fetch(1), move |_| {
                let _r = DropProbe(drops.clone());
                Async::map(fetch(1), move |_| {
                    assert_eq!(drops.load(Ordering::SeqCst), 0);
                })
            })
        })
            .async_();
        block_on_test(future);
    }

    // TEST UTILS

    // The closure will panic on call
    fn drop_spy_continuation_panic_on_call(
        drops: Arc<AtomicUsize>,
    ) -> impl ColoredCallOnce<Async, (), Value = ()> + CallOnce<()> {
        let capture = DropProbe(drops.clone());
        move |_| {
            closure(capture);
            Async::ready(())
        }
    }

    fn drop_spy_body_panic_on_call(
        drops: Arc<AtomicUsize>,
    ) -> impl ColoredThunkOnce<Async, Value = ()> + ThunkOnce {
        let capture = DropProbe(drops.clone());
        move || {
            closure(capture);
            Async::ready(())
        }
    }

    fn closure<T>(capture: T) {
        use_(&capture);
        panic!("Closure panic");
    }

    fn use_<T>(_v: &T) {
        black_box(_v);
    }

    fn test_spy() -> Arc<AtomicUsize> {
        Arc::new(AtomicUsize::new(0))
    }

    #[derive(Debug)]
    struct DropProbe(Arc<AtomicUsize>);

    impl Drop for DropProbe {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn pending_computation_drop_spy(drops: Arc<AtomicUsize>) -> impl Colored<Async, Output = ()> {
        <_ as Colorize<Async>>::colorize({
            let capture = DropProbe(drops);
            poll_fn(move |_| {
                // force the closure to keep it
                use_(&capture);
                Poll::<()>::Pending
            })
        })
    }

    fn ready_computation_drop_spy(drops: Arc<AtomicUsize>) -> impl Colored<Async, Output = ()> {
        <_ as Colorize<Async>>::colorize({
            let capture = DropProbe(drops);
            poll_fn(move |_| {
                // force the closure to keep it
                use_(&capture);
                Poll::Ready(())
            })
        })
    }

    fn ready_computation_panic_on_drop(
        drops: Arc<AtomicUsize>,
    ) -> impl Colored<Async, Output = ()> {
        struct ReadyPanicOnDrop {
            drops: Arc<AtomicUsize>,
        }

        impl Future for ReadyPanicOnDrop {
            type Output = ();

            fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
                Poll::Ready(())
            }
        }

        impl Drop for ReadyPanicOnDrop {
            fn drop(&mut self) {
                self.drops.fetch_add(1, Ordering::SeqCst);
                panic!("source drop panic");
            }
        }
        <_ as Colorize<Async>>::colorize(ReadyPanicOnDrop { drops })
    }
}

// cps already does this
#[cfg(test)]
mod pin_tests {
    use super::*;
    use crate::colors::async_::identity::impls::{AsyncExt, MakeColored};
    use crate::test_util::poll_once;
    use crate::{Cps, Loop, Ready, Wrap};
    use std::marker::PhantomPinned;
    use std::ops::ControlFlow;
    use std::pin::{pin, Pin};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::task::{Context, Poll};

    /// A `!Unpin` future that records where it was first polled and verifies
    /// that its destructor runs at that same address.
    struct AddressCheckedFuture<T> {
        output: Option<T>,
        yielded: bool,
        address: AtomicUsize,
        _pin: PhantomPinned,
    }

    impl<T> AddressCheckedFuture<T> {
        fn new(output: T) -> Self {
            Self {
                output: Some(output),
                yielded: false,
                address: AtomicUsize::default(),
                _pin: PhantomPinned,
            }
        }
    }

    impl<T> Future for AddressCheckedFuture<T> {
        type Output = T;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let current = self.as_ref().get_ref() as *const Self as usize;
            let recorded = self.address.load(Ordering::SeqCst);
            if recorded == 0 {
                self.address.store(current, Ordering::SeqCst);
            } else {
                assert_eq!(current, recorded, "pinned future moved between polls");
            }

            // SAFETY: only the ordinary fields are mutated. `_pin` is never
            // moved and `output` is consumed in place.
            let this = unsafe { self.as_mut().get_unchecked_mut() };
            if this.yielded {
                Poll::Ready(this.output.take().expect("future polled after completion"))
            } else {
                this.yielded = true;
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }

    impl<T> Drop for AddressCheckedFuture<T> {
        fn drop(&mut self) {
            let recorded = self.address.load(Ordering::SeqCst);
            if recorded != 0 {
                assert_eq!(
                    self as *const Self as usize, recorded,
                    "pinned future moved before being dropped"
                );
            }
        }
    }

    fn assert_func(future: impl Future<Output = usize>) {
        let mut future = pin!(future);
        assert!(poll_once(future.as_mut()).is_pending());
        assert_eq!(poll_once(future.as_mut()), Poll::Ready(5));
    }

    #[test]
    fn manual_combinators_preserve_the_address_of_pinned_flows() {
        {
            let future =
                Async::map(MakeColored(AddressCheckedFuture::new(4)), |value| value + 1).async_();
            assert_func(future)
        }

        {
            let future = Async::and_then(MakeColored(AddressCheckedFuture::new(4)), |value| {
                Async::ready(value + 1)
            })
                .async_();
            assert_func(future)
        }

        {
            let future =
                Async::wrap_flatten(move || MakeColored(AddressCheckedFuture::new(5))).async_();
            assert_func(future)
        }

        {
            let future = Async::fold((), {
                move |()| MakeColored(AddressCheckedFuture::new(ControlFlow::Break(5)))
            })
                .async_();
            assert_func(future)
        }
    }
}
