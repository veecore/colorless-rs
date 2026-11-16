use super::support::runtime;
use colorless_core::{Async, AsyncExt, Color, Colored, Colorize, OneOf2};
use criterion::Criterion;
use std::future::poll_fn;
use std::hint::black_box;
use std::ops::ControlFlow;
use std::task::Poll;

#[inline(never)]
fn future(i: i64) -> impl Future<Output = i64> {
    let mut countdown = 5;
    poll_fn(move |ctx| {
        if countdown == 0 {
            Poll::Ready(i)
        } else {
            countdown -= 1;
            ctx.waker().wake_by_ref();
            Poll::Pending
        }
    })
}

#[inline(never)]
async fn native_async_sum(n: i64) -> i64 {
    let mut sum = 0i64;
    let mut i = 0i64;

    while i < n {
        let v = future(i).await;
        sum += v;
        i += 1;
    }
    sum
}

trait Func<C> {
    fn return_value(i: i64) -> impl Colored<C, Output = i64>;
}

struct FImpl;

impl Func<Async> for FImpl {
    #[inline(always)]
    fn return_value(i: i64) -> impl Colored<Async, Output = i64> {
        <_ as Colorize<Async>>::colorize(future(i))
    }
}

#[inline(never)]
fn colored_fold_sum<C: Color, F: Func<C>>(n: i64) -> impl Colored<C, Output = i64> {
    C::wrap_flatten(move || {
        C::fold((n, 0i64, 0i64), move |(n, mut sum, mut i)| {
            if i < n {
                <C as OneOf2>::arm_0(C::map(F::return_value(i), move |v| {
                    sum += v;
                    i += 1;
                    ControlFlow::Continue((n, sum, i))
                }))
            } else {
                <C as OneOf2>::arm_1(C::ready(ControlFlow::Break(sum)))
            }
        })
    })
}

pub(crate) fn bench_async_loop(c: &mut Criterion) {
    let mut group = c.benchmark_group("loop_async_suspending");
    let runtime = runtime();

    for &size in &[10i64, 100i64] {
        group.bench_function(format!("native_async_sum_{size}"), |b| {
            b.to_async(&runtime)
                .iter(|| native_async_sum(black_box(size)))
        });

        group.bench_function(format!("async_fold_sum_{size}"), |b| {
            b.to_async(&runtime)
                .iter(|| colored_fold_sum::<Async, FImpl>(black_box(size)).async_())
        });
    }

    group.finish();
}
