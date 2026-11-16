#![allow(clippy::let_and_return)]

use colorless_core::{AsyncExt, IdentityExt, test_util::block_on_test};
use colorless_macros::colorless;
use criterion::{Criterion, criterion_group, criterion_main};
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

#[colorless]
#[inline(never)]
fn colored_single_map(x: i32) -> i32 {
    let a = colorless::ready(x + 1).resolve;
    a * 2
}

#[colorless]
#[inline(never)]
fn colored_single_and_then(x: i32) -> i32 {
    let a = colorless::ready(x + 1).resolve;
    colorless::ready(a * 2).resolve
}

#[colorless]
#[inline(never)]
fn colored_and_then_chain_5(x: i32) -> i32 {
    let a = colorless::ready(x + 1).resolve;
    let b = colorless::ready(a * 2).resolve;
    let c = colorless::ready(b - 3).resolve;
    let d = colorless::ready(c ^ 4).resolve;
    let e = colorless::ready(d + 5).resolve;
    e
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

    group.bench_function("async_and_then_chain_5", |b| {
        b.iter(|| block_on_test(colored_and_then_chain_5(black_box(42)).async_()))
    });

    group.finish();
}

criterion_group! {
    name = optimization;
    config = Criterion::default();
    targets =
        bench_sync_cps,
        bench_async_cps,
}

criterion_main!(optimization);
