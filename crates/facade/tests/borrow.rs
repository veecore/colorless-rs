#![cfg(feature = "relaxed-drop")]

use colorless_core::test_util::poll_once;
use colorless_core::{
    Async, AsyncExt, Color, Colored, Colorize, IdentityExt, ready, test_util::block_on_test,
};
use colorless_macros::{color, colorless};
use std::future::poll_fn;
use std::pin::pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::Poll;

async fn yield_once() {
    let mut yielded = false;
    poll_fn(move |context| {
        if yielded {
            Poll::Ready(())
        } else {
            yielded = true;
            context.waker().wake_by_ref();
            Poll::Pending
        }
    })
    .await;
}

fn suspend_once<T>(value: T) -> impl Colored<Async, Output = T> {
    <_ as Colorize<Async>>::colorize(async move {
        yield_once().await;
        value
    })
}

fn mutate_after_yield<'a>(value: &'a mut String) -> impl Colored<Async, Output = ()> + 'a {
    <_ as Colorize<Async>>::colorize(async move {
        yield_once().await;
        value.push('!');
    })
}

fn retains_borrow<C: Color, T>(value: &mut T) -> impl Colored<C, Output = ()> + use<'_, C, T> {
    C::wrap(move || {
        let _ = value;
    })
}

struct DropProbe(Arc<AtomicUsize>);

impl Drop for DropProbe {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
#[allow(unused_mut)]
fn local_pin_is_cleaned_up_when_an_async_future_is_cancelled() {
    let drops = Arc::new(AtomicUsize::new(0));
    let probe_drops = drops.clone();
    let computation = color! {
        let mut local = DropProbe(probe_drops);
        pin_local!(local);
        suspend_once(()).resolve;
        unpin_local!(local);
    };

    {
        let mut future = pin!(computation.async_());
        assert!(matches!(poll_once(future.as_mut()), Poll::Pending));
    }

    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[test]
#[allow(unused_mut)]
fn local_pin_is_cleaned_up_after_completion() {
    let drops = Arc::new(AtomicUsize::new(0));
    let probe_drops = drops.clone();
    let computation = color! {
        let mut local = DropProbe(probe_drops);
        pin_local!(local);
        suspend_once(()).resolve;
        unpin_local!(local);
    };

    block_on_test(computation.async_());
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

/// The borrowed local remains physically stable while the inner Async state
/// machine is pending. Once that borrowed computation completes, ownership is
/// restored before the next source statement observes `local` by value.
#[test]
fn local_pin_supports_a_real_mut_borrow_across_suspension() {
    let computation = color! {
        let mut local = String::from("stable");
        pin_local!(local);
        mutate_after_yield(local).resolve;
        unpin_local!(local);
        local
    };

    assert_eq!(block_on_test(computation.async_()), "stable!");
}

#[test]
fn local_pin_supports_double_pin() {
    fn is_mut<T>(_t: &mut T) {}

    #[colorless]
    fn run() {
        let mut local = String::from("local");
        pin_local!(local);
        ready(()).resolve;
        // this is just pinning &mut String
        pin_local!(local);
        ready(()).resolve;
        unpin_local!(local);
        is_mut(local);
        // the actual storage is still cleaned
    }

    run().sync_()
}

#[test]
fn unexplicitly_unpinned_is_dropped() {
    #[colorless]
    fn run(d: Arc<AtomicUsize>) {
        let mut d = DropProbe(d);
        pin_local!(d);
        ready(()).resolve;
    }

    let drops = Arc::new(AtomicUsize::new(0));
    let probe_drops = drops.clone();
    run(probe_drops).sync_();
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[test]
fn pin_crossing_multiple_paths_is_dropped() {
    #[colorless]
    fn run(d: Arc<AtomicUsize>, take_left: bool) {
        let mut a = DropProbe(d);
        pin_local!(a);
        ready(()).resolve;
        if take_left {
            ready(()).resolve;
        } else {
            // unpin to test that it's not dropped for all
            unpin_local!(a);
            drop(a);
        }
    }

    let drops = Arc::new(AtomicUsize::new(0));
    for t in [true, false] {
        run(drops.clone(), t).sync_();
        assert_eq!(drops.load(Ordering::SeqCst), 1);

        block_on_test(run(drops.clone(), t).async_());
        assert_eq!(drops.load(Ordering::SeqCst), 2);

        drops.fetch_sub(2, Ordering::SeqCst);
    }
}

#[test]
fn pin_can_cross_multiple_resolves() {
    #[colorless]
    fn run() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut d = DropProbe(drops.clone());
        pin_local!(d);
        ready(()).resolve;
        ready(()).resolve;
        ready(()).resolve;
        ready(()).resolve;
        // check that it's still valid
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        unpin_local!(d);
        assert_eq!(drops.load(Ordering::SeqCst), 0);
    }

    run().sync_();
    block_on_test(run().async_())
}

#[test]
fn pin_allows_use_across_multiple_resolves() {
    #[allow(unused_mut)]
    #[colorless]
    fn run(mut a: String) -> String {
        pin_local!(a);
        ready(()).resolve;
        a.push_str("-first");
        ready(()).resolve;
        a.push_str("-second");
        ready(()).resolve;
        a.push_str("-third");
        ready(()).resolve;
        unpin_local!(a);
        a
    }

    assert_eq!(
        run(String::from("start")).sync_(),
        "start-first-second-third"
    );
    assert_eq!(
        block_on_test(run(String::from("start")).async_()),
        "start-first-second-third"
    );
}

#[test]
fn pin_allows_independent_unpin() {
    #[colorless]
    fn run() {
        let drops_a = Arc::new(AtomicUsize::new(0));
        let mut a = DropProbe(drops_a.clone());
        pin_local!(a);

        let drops_b = Arc::new(AtomicUsize::new(0));
        let mut b = DropProbe(drops_b.clone());
        pin_local!(b);

        ready(()).resolve;
        unpin_local!(a);

        assert_eq!(drops_a.load(Ordering::SeqCst), 0);
        assert_eq!(drops_b.load(Ordering::SeqCst), 0);
        drop(a);

        unpin_local!(b);
        assert_eq!(drops_a.load(Ordering::SeqCst), 1);
        assert_eq!(drops_b.load(Ordering::SeqCst), 0);
    }

    run().sync_();
    block_on_test(run().async_())
}

// same as pin_crossing_multiple_paths_is_dropped
#[test]
fn pin_crossing_early_abort_is_unpinned() {
    #[colorless]
    fn run(drops: Arc<AtomicUsize>) -> Result<(), ()> {
        let mut a = DropProbe(drops.clone());
        pin_local!(a);
        // fixme: type annotation issue
        ready(Err::<(), ()>(())).resolve?;
        unpin_local!(a);
        Ok(())
    }

    let drops = Arc::new(AtomicUsize::new(0));

    run(drops.clone()).sync_().unwrap_err();
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    block_on_test(run(drops.clone()).async_()).unwrap_err();
    assert_eq!(drops.load(Ordering::SeqCst), 2);
}

#[test]
fn pin_crossing_loop_is_dropped() {
    #[colorless]
    fn run(drops: Arc<AtomicUsize>) {
        let mut a = DropProbe(drops.clone());
        pin_local!(a);

        let mut i = 0;
        #[loop_state(mut i, a, drops)]
        while i < 3 {
            ready(()).resolve;
            let __ref = &a;
            i += 1;
            assert_eq!(drops.load(Ordering::SeqCst), 0);
        }
        // yeah, loop is still an issue
        // unpin_local!(a);
    }

    let drops = Arc::new(AtomicUsize::new(0));
    run(drops.clone()).sync_();
    assert_eq!(drops.load(Ordering::SeqCst), 1);

    drops.fetch_sub(1, Ordering::SeqCst);
    block_on_test(run(drops.clone()).async_());
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[test]
fn pin_can_end_after_a_branch() {
    #[colorless]
    fn run(c: bool) {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut a = DropProbe(drops.clone());
        pin_local!(a);
        ready(()).resolve;
        if c {
            ready(()).resolve;
        }
        unpin_local!(a);
        assert_eq!(drops.load(Ordering::SeqCst), 0);
    }

    for t in [true, false] {
        run(t).sync_();
        block_on_test(run(t).async_());
    }
}

#[test]
fn reborrow_pin_works() {
    #[colorless]
    fn run() {
        let mut local = 0;
        pin_local!(local);
        borrow_mut!(local);

        retains_borrow(reborrow!(local)).resolve;

        end_borrow_mut!(local);
        unpin_local!(local);
    }

    run().sync_()
}

#[test]
fn reborrow_allows_linear_borrow_across_resolve() {
    #[colorless]
    fn run(b: &mut String) {
        borrow_mut!(b);
        retains_borrow(reborrow!(b)).resolve;
        let _ = reborrow!(b);
    }
}

#[test]
fn loop_state_reborrow() {
    #[colorless]
    fn push_str(s: &mut String) {
        s.push_str("Hello, world!")
    }

    #[colorless]
    pub fn run(v: &mut String) {
        borrow_mut!(v);

        #[colorless::loop_state(mut v)]
        for _ in 0..100 {
            push_str(reborrow!(v)).resolve;
        }
    }

    let mut env = String::from("Hello, world!");

    run(&mut env).sync_();
    assert_eq!(env, "Hello, world!".repeat(101));
}
