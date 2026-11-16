use colorless_core::{AsyncExt, Color, Colored, IdentityExt, test_util::block_on_test};
use criterion::Criterion;
use std::hint::black_box;
// ── Native Baselines ─────────────────────────────────────────────────────────

#[inline(never)]
fn native_sync_single(x: i32) -> i32 {
    (x + 1) * 2
}

#[inline(never)]
fn native_sync_chain_5(x: i32) -> i32 {
    let a = x + 1;
    let b = a * 2;
    let c = b - 3;
    let d = c ^ 4;
    d + 5
}

#[inline(never)]
async fn native_async_single(x: i32) -> i32 {
    let a = std::future::ready(x + 1).await;
    a * 2
}

#[inline(never)]
async fn native_async_chain_5(x: i32) -> i32 {
    let a = std::future::ready(x + 1).await;
    let b = std::future::ready(a * 2).await;
    let c = std::future::ready(b - 3).await;
    let d = std::future::ready(c ^ 4).await;
    let e = std::future::ready(d + 5).await;
    e
}

// ── Generic Colored CPS Pipelines ───────────────────────────────────────────

#[inline(never)]
fn colored_single_map<C: Color>(x: i32) -> impl Colored<C, Output = i32> {
    C::wrap_flatten(move || C::map(C::ready(x + 1), move |a| a * 2))
}

#[inline(never)]
fn colored_single_and_then<C: Color>(x: i32) -> impl Colored<C, Output = i32> {
    C::wrap_flatten(move || C::and_then(C::ready(x + 1), move |a| C::ready(a * 2)))
}

#[inline(never)]
fn colored_map_chain_5<C: Color>(x: i32) -> impl Colored<C, Output = i32> {
    C::wrap_flatten(move || {
        let c1 = C::map(C::ready(x), |v| v + 1);
        let c2 = C::map(c1, |v| v * 2);
        let c3 = C::map(c2, |v| v - 3);
        let c4 = C::map(c3, |v| v ^ 4);
        C::map(c4, |v| v + 5)
    })
}

#[inline(never)]
fn colored_and_then_chain_5<C: Color>(x: i32) -> impl Colored<C, Output = i32> {
    C::wrap_flatten(move || {
        C::and_then(C::ready(x), |v1| {
            C::and_then(C::ready(v1 + 1), |v2| {
                C::and_then(C::ready(v2 * 2), |v3| {
                    C::and_then(C::ready(v3 - 3), |v4| {
                        C::and_then(C::ready(v4 ^ 4), |v5| C::ready(v5 + 5))
                    })
                })
            })
        })
    })
}

// ── Benchmarks: Sync (Identity) ─────────────────────────────────────────────

pub(crate) fn bench_sync_cps(c: &mut Criterion) {
    let mut group = c.benchmark_group("cps_sync");

    // Single step
    group.bench_function("native_sync_single", |b| {
        b.iter(|| native_sync_single(black_box(42)))
    });

    group.bench_function("identity_single_map", |b| {
        b.iter(|| colored_single_map(black_box(42)).sync_())
    });

    group.bench_function("identity_single_and_then", |b| {
        b.iter(|| colored_single_and_then(black_box(42)).sync_())
    });

    // 5-step pipeline
    group.bench_function("native_sync_chain_5", |b| {
        b.iter(|| native_sync_chain_5(black_box(42)))
    });

    group.bench_function("identity_map_chain_5", |b| {
        b.iter(|| colored_map_chain_5(black_box(42)).sync_())
    });

    group.bench_function("identity_and_then_chain_5", |b| {
        b.iter(|| colored_and_then_chain_5(black_box(42)).sync_())
    });

    group.finish();
}

// ── Benchmarks: Async ───────────────────────────────────────────────────────

pub(crate) fn bench_async_cps(c: &mut Criterion) {
    let mut group = c.benchmark_group("cps_async");

    // Single step
    group.bench_function("native_async_single", |b| {
        b.iter(|| block_on_test(native_async_single(black_box(42))))
    });

    group.bench_function("async_single_map", |b| {
        b.iter(|| block_on_test(colored_single_map(black_box(42)).async_()))
    });

    group.bench_function("async_single_and_then", |b| {
        b.iter(|| block_on_test(colored_single_and_then(black_box(42)).async_()))
    });

    // 5-step pipeline
    group.bench_function("native_async_chain_5", |b| {
        b.iter(|| block_on_test(native_async_chain_5(black_box(42))))
    });

    group.bench_function("async_map_chain_5", |b| {
        b.iter(|| block_on_test(colored_map_chain_5(black_box(42)).async_()))
    });

    group.bench_function("async_and_then_chain_5", |b| {
        b.iter(|| block_on_test(colored_and_then_chain_5(black_box(42)).async_()))
    });

    group.finish();
}
