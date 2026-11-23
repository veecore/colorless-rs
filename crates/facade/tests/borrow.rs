#![cfg(feature = "relaxed-drop")]

use colorless_core::test_util::{block_on_local_test, poll_once};
use colorless_core::{
    Async, AsyncExt, Color, Colored, Identity, IdentityExt, ready, test_util::block_on_test,
};
use colorless_macros::{color, colorless};
use colorless_std::io::ColoredWrite;
use std::error::Error;
use std::fmt::Display;
use std::future::poll_fn;
use std::ops::DerefMut;
use std::pin::{Pin, pin};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll};

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

#[colorless(color = Async)]
async fn suspend_once<T>(value: T) -> T {
    yield_once().await;
    value
}

#[colorless(color = Async)]
async fn mutate_after_yield<'a>(value: &'a mut String) {
    yield_once().await;
    value.push('!');
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
        pin!(local);
        suspend_once(()).resolve;
        unpin!(local);
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
        pin!(local);
        suspend_once(()).resolve;
        unpin!(local);
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
        pin!(local);
        mutate_after_yield(rb!(local)).resolve;
        unpin!(local);
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
        pin!(local);
        ready(()).resolve;
        pin!(local);
        ready(()).resolve;
        unpin!(local);
        is_mut(rb!(local));
    }

    run().sync_()
}

#[test]
fn unexplicitly_unpinned_is_dropped() {
    #[colorless]
    fn run(d: Arc<AtomicUsize>) {
        let mut d = DropProbe(d);
        pin!(d);
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
        pin!(a);
        ready(()).resolve;
        if take_left {
            ready(()).resolve;
        } else {
            // unpin to test that it's not dropped for all
            unpin!(a);
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
        pin!(d);
        ready(()).resolve;
        ready(()).resolve;
        ready(()).resolve;
        ready(()).resolve;
        // check that it's still valid
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        unpin!(d);
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
        pin!(a);
        ready(()).resolve;
        rb!(a).push_str("-first");
        ready(()).resolve;
        rb!(a).push_str("-second");
        ready(()).resolve;
        rb!(a).push_str("-third");
        ready(()).resolve;
        unpin!(a);
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
        pin!(a);

        let drops_b = Arc::new(AtomicUsize::new(0));
        let mut b = DropProbe(drops_b.clone());
        pin!(b);

        ready(()).resolve;
        unpin!(a);

        assert_eq!(drops_a.load(Ordering::SeqCst), 0);
        assert_eq!(drops_b.load(Ordering::SeqCst), 0);
        drop(a);

        unpin!(b);
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
        pin!(a);
        // fixme: type annotation issue
        ready(Err::<(), ()>(())).resolve?;
        unpin!(a);
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
        pin!(a);

        let mut i = 0;
        #[state(mut i, a, drops)]
        while i < 3 {
            ready(()).resolve;
            let __ref = &a;
            i += 1;
            assert_eq!(drops.load(Ordering::SeqCst), 0);
        }
        // yeah, loop is still an issue
        // unpin!(a);
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
        pin!(a);
        ready(()).resolve;
        if c {
            ready(()).resolve;
        }
        unpin!(a);
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
        pin!(local);
        retains_borrow(rb!(local)).resolve;
        unpin!(local);
    }

    run().sync_()
}

#[test]
fn reborrow_allows_linear_borrow_across_resolve() {
    #[colorless]
    fn run(b: &mut String) -> &mut String {
        borrow!(b);
        retains_borrow(rb!(b)).resolve;
        let _ = rb!(b);
        rb!(b)
    }

    let mut binding = "df".to_string();
    let f = block_on_test(run(&mut binding).async_());
    f.push('d');
}

#[test]
fn loop_state_reborrow() {
    #[colorless]
    fn push_str(s: &mut String) {
        s.push_str("Hello, world!")
    }

    #[colorless]
    pub fn run(v: &mut String) {
        borrow!(v);

        #[colorless::state(mut v)]
        for _ in 0..100 {
            push_str(rb!(v)).resolve;
        }
    }

    let mut env = String::from("Hello, world!");

    run(&mut env).sync_();
    assert_eq!(env, "Hello, world!".repeat(101));
}

#[test]
fn iter() {
    #[colorless]
    fn serialize<I: Display>(i: &I) {
        eprintln!("{}", i);
    }

    #[colorless]
    fn collect_seq<'a, I>(iter: I)
    where
        I: IntoIterator + 'a,
        <I as IntoIterator>::Item: Display,
    {
        for mut item in iter {
            pin!(item);
            serialize(rb!(item)).resolve;
        }
    }

    {
        let i = String::from("Hello, world!");
        let i = &i.as_str();
        let mut iter = std::iter::once(i);
        collect_seq(&mut iter).sync_();
        // exhausted here but the point is to check non-static requirement
        collect_seq(iter.copied()).sync_();
    }
}

#[test]
fn loop_reborrow() {
    #[colorless(C)]
    fn run(w: &mut impl ColoredWrite<C>, count: usize) -> Result<(), Box<dyn Error>> {
        borrow!(w);

        #[colorless::state(mut w)]
        for _ in 0..count {
            rb!(w).write_all(b"hello world!").resolve.unwrap();
        }
        return Ok(());
    }

    struct Sink {
        v: Vec<u8>,
    }

    #[colorless(color = Async)]
    impl ColoredWrite for Sink {
        async fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            let mut count = 0;
            poll_fn(move |cx| {
                if count >= 200 {
                    let half = b.len().div_ceil(2);
                    self.v.extend_from_slice(&b[..half]);
                    return Poll::Ready(Ok(half));
                }
                count += 1;
                cx.waker().wake_by_ref();
                Poll::Pending
            })
            .await
        }

        async fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[colorless(color = Identity)]
    impl ColoredWrite for Sink {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            let mut count = 0;
            loop {
                if count >= 200 {
                    let half = b.len().div_ceil(2);
                    self.v.extend_from_slice(&b[..half]);
                    return Ok(half);
                }
                count += 1;
            }
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let mut s = Sink { v: Vec::new() };

    run(&mut s, 2).sync_().unwrap();
    assert_eq!(s.v, b"hello world!".repeat(2));

    s.v.clear();
    block_on_local_test(run(&mut s, 2).async_()).unwrap();
    assert_eq!(s.v, b"hello world!".repeat(2));
}

#[test]
#[allow(dead_code)]
fn pin_reborrow() {
    #[colorless(Z)]
    pub trait Serializer {
        type SerializeTuple: SerializeTuple<Z>;

        fn serialize_tuple(self) -> Self::SerializeTuple;
    }

    #[colorless(Z)]
    pub trait SerializeTuple {
        fn serialize_nothing(&mut self);
    }

    #[colorless(Z)]
    fn serialize<S>(serializer: S)
    where
        S: Serializer<Z>,
    {
        let mut tuple = serializer.serialize_tuple().resolve;

        pin!(tuple);
        rb!(tuple).serialize_nothing().resolve;

        unpin!(tuple);
    }
}

#[test]
#[cfg_attr(
    miri,
    ignore = "Known limitation: Stacked Borrows strongly protects \
    &mut closure arguments across nested CPS continuations"
)]
fn double_borrow() {
    fn first_reference_twice<C: Color>(x: &mut String) -> impl Colored<C, Output = &mut str> {
        C::wrap(move || x.deref_mut())
    }

    fn last_reference_single<C: Color>(_x: &mut String) -> impl Colored<C, Output = ()> {
        _x.push_str("!");
        C::ready(())
    }

    #[colorless]
    fn run() {
        let mut x = String::from("Hello, world!");
        pin!(x);
        let o = first_reference_twice(rb!(x)).resolve;
        o.make_ascii_lowercase();
        last_reference_single(rb!(x)).resolve;
        unpin!(x);

        assert_eq!(x, "hello, world!!");
    }

    run().sync_();
    block_on_test(run().async_());
}

#[test]
fn color_sees_borrow() {
    #[colorless(Z)]
    fn run() {
        let mut x = String::from("Hello, world!");
        pin!(x);
        let y = rbv!(x);
        color! {
            #![colorless(no_wrap, color = Z)]

            ready(()).resolve;
            y.make_ascii_lowercase();
            ready(()).resolve;
        }
        .resolve;

        rb!(x).make_ascii_lowercase();

        ready(()).resolve;
    }

    run().sync_();
    block_on_test(run().async_());

    async fn run_async() {
        let mut x = String::from("Hello, world!");
        let y = &mut x;
        async {
            y.make_ascii_lowercase();
        }
        .await;
        x.make_ascii_lowercase();
        std::future::ready(()).await
    }

    block_on_test(run_async());
}

#[test]
#[allow(dead_code)]
fn disjoint_borrow() {
    struct Struct {
        a: String,
        b: i32,
    }

    fn push(a: &mut String, b: &mut i32) {
        a.push_str(b.to_string().as_str());
    }

    #[colorless]
    fn run(s: &mut Struct) {
        borrow!(s);
        push(&mut rb!(s).a, &mut rb!(s).b);
    }

    let mut s = Some(0u8);
    let mut v = Vec::new();
    v.extend(s.take());
}

// BUG: lower causes delay in drop
#[test]
#[cfg_attr(miri, ignore)]
fn borrow_drop_test() {
    struct HoldOn<'a>(&'a mut String);

    impl<'a> Drop for HoldOn<'a> {
        fn drop(&mut self) {
            self.0.make_ascii_lowercase();
        }
    }

    #[colorless]
    fn run(s: &mut String) {
        borrow!(s);

        {
            let _d = HoldOn(rb!(s));
            // drop(_d); // if d were a resolved colored computation, it'd already be dropped
            // by the color
            ready(()).resolve;
        }

        rb!(s).make_ascii_uppercase();
    }

    let mut s = String::from("Hello, world!");
    run(&mut s).sync_();
    assert_eq!(s.as_str(), "HELLO, WORLD!");
}

#[test]
fn borrow_drop_test_computation() {
    struct HoldOn<'a>(&'a mut String);

    impl<'a> Drop for HoldOn<'a> {
        fn drop(&mut self) {
            self.0.make_ascii_lowercase();
        }
    }

    impl<'a> Future for HoldOn<'a> {
        type Output = ();

        fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
            Poll::Ready(())
        }
    }

    #[colorless(color = Async)]
    fn comp(s: &mut String) {
        HoldOn(s)
    }

    let mut a = String::from("Hello, world!");

    let s = &mut a;
    let f = color! {
        #![colorless(color = Async)]
        borrow!(s);

        {
            let _d = comp(rb!(s)).resolve;

            ready(()).resolve;
        }

        rb!(s).make_ascii_uppercase();
    };

    block_on_test(f.async_());

    assert_eq!(a.as_str(), "HELLO, WORLD!");
}
