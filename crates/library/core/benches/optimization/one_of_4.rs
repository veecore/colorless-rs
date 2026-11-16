use colorless_core::{AsyncExt, Color, Colored, IdentityExt, OneOf4, test_util::block_on_test};
use criterion::Criterion;
use std::hint::black_box;
// ── Native leaf ──────────────────────────────────────────────────────────────

#[inline(never)]
async fn native_leaf(v: i32) -> i32 {
    v * 10
}

// ── Native match ─────────────────────────────────────────────────────────────

#[inline(never)]
fn native_sync_match(x: u8, a: i32, b: i32, c: i32, d: i32) -> i32 {
    match x {
        0 => a * 10,
        1 => b * 10,
        2 => c * 10,
        _ => d * 10,
    }
}

#[inline(never)]
async fn native_async_match(x: u8, a: i32, b: i32, c: i32, d: i32) -> i32 {
    match x {
        0 => native_leaf(a).await,
        1 => native_leaf(b).await,
        2 => native_leaf(c).await,
        _ => native_leaf(d).await,
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

#[inline(never)]
fn fetch<C: Color>(v: i32) -> impl Colored<C, Output = i32> {
    C::ready(v * 10)
}

// ── Colored Match4 (general, supports heterogeneous arm types) ───────────────

#[inline(never)]
fn colored_match4<C: Color>(
    x: u8,
    a: i32,
    b: i32,
    c: i32,
    d: i32,
) -> impl Colored<C, Output = i32> {
    C::wrap_flatten(move || match x {
        0 => <C as OneOf4>::arm_0(fetch(a)),
        1 => <C as OneOf4>::arm_1(fetch(b)),
        2 => <C as OneOf4>::arm_2(fetch(c)),
        _ => <C as OneOf4>::arm_3(fetch(d)),
    })
}

// ── Bench groups ─────────────────────────────────────────────────────────────
pub(crate) fn bench_sync_match(c: &mut Criterion) {
    let mut group = c.benchmark_group("sync_match");

    group.bench_function("native_match", |b| {
        let mut x: u8 = 0;
        b.iter(|| {
            x = x.wrapping_add(1) % 4;
            native_sync_match(
                black_box(x),
                black_box(1),
                black_box(2),
                black_box(3),
                black_box(4),
            )
        });
    });

    group.bench_function("colored_match4", |b| {
        let mut x: u8 = 0;
        b.iter(|| {
            x = x.wrapping_add(1) % 4;
            colored_match4(
                black_box(x),
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

pub(crate) fn bench_sync_match_const_arms(c: &mut Criterion) {
    let mut group = c.benchmark_group("sync_match_const_arms");

    group.bench_function("native_match", |b| {
        let mut x: u8 = 0;
        b.iter(|| {
            x = x.wrapping_add(1) % 4;
            native_sync_match(black_box(x), 1, 2, 3, 4)
        });
    });

    group.bench_function("colored_match4", |b| {
        let mut x: u8 = 0;
        b.iter(|| {
            x = x.wrapping_add(1) % 4;
            colored_match4(black_box(x), 1, 2, 3, 4).sync_()
        });
    });

    group.finish();
}

pub(crate) fn bench_async_match(c: &mut Criterion) {
    let mut group = c.benchmark_group("async_match");

    group.bench_function("native_async_match", |b| {
        let mut x: u8 = 0;
        b.iter(|| {
            x = x.wrapping_add(1) % 4;
            block_on_test(native_async_match(
                black_box(x),
                black_box(1),
                black_box(2),
                black_box(3),
                black_box(4),
            ))
        });
    });

    group.bench_function("colored_match4", |b| {
        let mut x: u8 = 0;
        b.iter(|| {
            x = x.wrapping_add(1) % 4;
            block_on_test(
                colored_match4(
                    black_box(x),
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

pub(crate) fn bench_async_match_const_arms(c: &mut Criterion) {
    let mut group = c.benchmark_group("async_match_const_arms");

    group.bench_function("native_async_match", |b| {
        let mut x: u8 = 0;
        b.iter(|| {
            x = x.wrapping_add(1) % 4;
            block_on_test(native_async_match(black_box(x), 1, 2, 3, 4))
        });
    });

    group.bench_function("colored_match4", |b| {
        let mut x: u8 = 0;
        b.iter(|| {
            x = x.wrapping_add(1) % 4;
            block_on_test(colored_match4(black_box(x), 1, 2, 3, 4).async_())
        });
    });

    group.finish();
}
