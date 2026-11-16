#![allow(clippy::too_many_arguments)]

use colorless_core::{AsyncExt, IdentityExt, test_util::block_on_test};
use colorless_macros::colorless;
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
// ── Native leaf ──────────────────────────────────────────────────────────────

#[inline(never)]
async fn native_leaf(v: i32) -> i32 {
    v * 10
}

// ── Native match ─────────────────────────────────────────────────────────────

#[inline(never)]
fn native_sync_match(
    x: u8,
    a: i32,
    b: i32,
    c: i32,
    d: i32,
    e: i32,
    f: i32,
    g: i32,
    h: i32,
    i: i32,
    j: i32,
    k: i32,
    l: i32,
) -> i32 {
    match x {
        0 => a * 10,
        1 => b * 10,
        2 => c * 10,
        3 => d * 10,
        4 => e * 10,
        5 => f * 10,
        6 => g * 10,
        7 => h * 10,
        8 => i * 10,
        9 => j * 10,
        10 => k * 10,
        11 => l * 10,
        _ => 10,
    }
}

#[inline(never)]
async fn native_async_match(
    x: u8,
    a: i32,
    b: i32,
    c: i32,
    d: i32,
    e: i32,
    f: i32,
    g: i32,
    h: i32,
    i: i32,
    j: i32,
    k: i32,
    l: i32,
) -> i32 {
    match x {
        0 => native_leaf(a).await,
        1 => native_leaf(b).await,
        2 => native_leaf(c).await,
        3 => native_leaf(d).await,
        4 => native_leaf(e).await,
        5 => native_leaf(f).await,
        6 => native_leaf(g).await,
        7 => native_leaf(h).await,
        8 => native_leaf(i).await,
        9 => native_leaf(j).await,
        10 => native_leaf(k).await,
        11 => native_leaf(l).await,
        _ => 10,
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

#[colorless]
#[inline(never)]
fn fetch(v: i32) -> i32 {
    v * 10
}

// ── Colored Match4 (general, supports heterogeneous arm types) ───────────────

#[colorless]
#[inline(never)]
fn colored_match4_visitor(
    x: u8,
    a: i32,
    b: i32,
    c: i32,
    d: i32,
    e: i32,
    f: i32,
    g: i32,
    h: i32,
    i: i32,
    j: i32,
    k: i32,
    l: i32,
) -> i32 {
    match x {
        0 => fetch(a).resolve,
        1 => fetch(b).resolve,
        2 => fetch(c).resolve,
        3 => fetch(d).resolve,
        4 => fetch(e).resolve,
        5 => fetch(f).resolve,
        6 => fetch(g).resolve,
        7 => fetch(h).resolve,
        8 => fetch(i).resolve,
        9 => fetch(j).resolve,
        10 => fetch(k).resolve,
        11 => fetch(l).resolve,
        _ => 10,
    }
}

// ── Bench groups ─────────────────────────────────────────────────────────────

pub(crate) fn bench_sync_match(c: &mut Criterion) {
    let mut group = c.benchmark_group("sync_match");

    group.bench_function("native_match_12", |b| {
        let mut x: u8 = 0;
        b.iter(|| {
            x = x.wrapping_add(1) % 4;
            native_sync_match(
                black_box(x),
                black_box(1),
                black_box(2),
                black_box(3),
                black_box(4),
                black_box(1),
                black_box(2),
                black_box(3),
                black_box(4),
                black_box(1),
                black_box(2),
                black_box(3),
                black_box(4),
            )
        });
    });

    group.bench_function("colored_match_12", |b| {
        let mut x: u8 = 0;
        b.iter(|| {
            x = x.wrapping_add(1) % 4;
            colored_match4_visitor(
                black_box(x),
                black_box(1),
                black_box(2),
                black_box(3),
                black_box(4),
                black_box(1),
                black_box(2),
                black_box(3),
                black_box(4),
                black_box(1),
                black_box(2),
                black_box(3),
                black_box(4),
            )
            .sync_()
        });
    });

    group.finish();
}

pub(crate) fn bench_async_match(c: &mut Criterion) {
    let mut group = c.benchmark_group("async_match");

    group.bench_function("native_async_match_12", |b| {
        let mut x: u8 = 0;
        b.iter(|| {
            x = x.wrapping_add(1) % 12;
            block_on_test(native_async_match(
                black_box(x),
                black_box(1),
                black_box(2),
                black_box(3),
                black_box(4),
                black_box(1),
                black_box(2),
                black_box(3),
                black_box(4),
                black_box(1),
                black_box(2),
                black_box(3),
                black_box(4),
            ))
        });
    });

    group.bench_function("colored_match_12", |b| {
        let mut x: u8 = 0;
        b.iter(|| {
            x = x.wrapping_add(1) % 12;
            block_on_test(
                colored_match4_visitor(
                    black_box(x),
                    black_box(1),
                    black_box(2),
                    black_box(3),
                    black_box(4),
                    black_box(1),
                    black_box(2),
                    black_box(3),
                    black_box(4),
                    black_box(1),
                    black_box(2),
                    black_box(3),
                    black_box(4),
                )
                .async_(),
            )
        });
    });

    group.finish();
}

criterion_group! {
    name = optimization;
    config = Criterion::default();
    targets =
        bench_sync_match,
        bench_async_match,
}

criterion_main!(optimization);
