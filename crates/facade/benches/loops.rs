use colorless_core::{AsyncExt, IdentityExt, test_util::block_on_test};
use colorless_macros::colorless;
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
// ── Native Baselines ─────────────────────────────────────────────────────────

#[inline(never)]
fn native_for_sum(n: i64) -> i64 {
    let mut sum = 0i64;
    for i in 0..n {
        sum += i;
    }
    sum
}

#[inline(never)]
fn native_while_sum(n: i64) -> i64 {
    let mut sum = 0i64;
    let mut i = 0i64;
    while i < n {
        sum += i;
        i += 1;
    }
    sum
}

#[inline(never)]
async fn native_async_sum(n: i64) -> i64 {
    let mut sum = 0i64;
    let mut i = 0i64;
    while i < n {
        let v = std::future::ready(i).await;
        sum += v;
        i += 1;
    }
    sum
}

#[inline(never)]
async fn native_async_sum_ref(acc: &mut i64, n: i64) -> i64 {
    let mut i = 0i64;
    while i < n {
        let v = std::future::ready(i).await;
        *acc += v;
        i += 1;
    }
    *acc
}

// ── Generic Colored Loops (Runtime-agnostic) ────────────────────────────────

/// Pure state-threading loop: passes accumulator value `(sum, i)` between iterations.
#[colorless]
#[inline(never)]
fn colored_fold_sum(n: i64) -> i64 {
    let mut sum = 0i64;
    let mut i = 0i64;
    #[state(pub mut sum, mut i, n)]
    while i < n {
        let v = colorless::ready(i).resolve;
        sum += v;
        i += 1;
    }
    sum
}

/// Reference-threading loop: passes a mutable reference `(&mut acc, i)` across iterations.
#[colorless]
#[inline(never)]
fn colored_fold_ref_sum(acc: &mut i64, n: i64) -> i64 {
    let mut i = 0i64;
    #[state(mut acc, mut i, n)]
    // while couldn't be used due to the current threading issue in codegen
    loop {
        if i < n {
            let v = colorless::ready(i).resolve;
            *acc += v;
            i += 1;
        } else {
            break *acc;
        }
    }
}

// ── Benchmarks: Sync (Identity) ─────────────────────────────────────────────

pub(crate) fn bench_sync_loop(c: &mut Criterion) {
    let mut group = c.benchmark_group("loop_sync");

    for &size in &[10i64, 1000i64] {
        group.bench_function(format!("native_for_sum_{size}"), |b| {
            b.iter(|| native_for_sum(black_box(size)))
        });

        group.bench_function(format!("native_while_sum_{size}"), |b| {
            b.iter(|| native_while_sum(black_box(size)))
        });

        group.bench_function(format!("identity_fold_sum_{size}"), |b| {
            b.iter(|| colored_fold_sum(black_box(size)).sync_())
        });

        group.bench_function(format!("identity_fold_ref_sum_{size}"), |b| {
            b.iter(|| {
                let mut acc = 0i64;
                colored_fold_ref_sum(&mut acc, black_box(size)).sync_()
            })
        });
    }

    group.finish();
}

// ── Benchmarks: Async ───────────────────────────────────────────────────────

pub(crate) fn bench_async_loop(c: &mut Criterion) {
    let mut group = c.benchmark_group("loop_async");

    for &size in &[10i64, 100i64] {
        group.bench_function(format!("native_async_sum_{size}"), |b| {
            b.iter(|| block_on_test(native_async_sum(black_box(size))))
        });

        group.bench_function(format!("native_async_sum_ref{size}"), |b| {
            b.iter(|| {
                let mut acc = 0i64;
                block_on_test(native_async_sum_ref(&mut acc, black_box(size)))
            })
        });

        group.bench_function(format!("async_fold_sum_{size}"), |b| {
            b.iter(|| block_on_test(colored_fold_sum(black_box(size)).async_()))
        });

        group.bench_function(format!("async_fold_ref_sum_{size}"), |b| {
            b.iter(|| {
                let mut acc = 0i64;
                block_on_test(colored_fold_ref_sum(&mut acc, black_box(size)).async_())
            })
        });
    }

    group.finish();
}

criterion_group! {
    name = optimization;
    config = Criterion::default();
    targets =
        bench_sync_loop,
        bench_async_loop,
}

criterion_main!(optimization);
