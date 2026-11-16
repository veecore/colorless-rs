use colorless_core::{AsyncExt, Color, Colored, IdentityExt, OneOf12, test_util::block_on_test};
use criterion::Criterion;
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

#[inline(never)]
fn fetch<C: Color>(v: i32) -> impl Colored<C, Output = i32> {
    C::ready(v * 10)
}

// ── Colored Match4 (general, supports heterogeneous arm types) ───────────────

#[inline(never)]
fn colored_match4_visitor<C: Color>(
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
) -> impl Colored<C, Output = i32> {
    C::wrap_flatten(move || match x {
        0 => <C as OneOf12>::arm_0(fetch::<C>(a)),
        1 => <C as OneOf12>::arm_1(fetch::<C>(b)),
        2 => <C as OneOf12>::arm_2(fetch::<C>(c)),
        3 => <C as OneOf12>::arm_3(fetch::<C>(d)),
        4 => <C as OneOf12>::arm_4(fetch::<C>(e)),
        5 => <C as OneOf12>::arm_5(fetch::<C>(f)),
        6 => <C as OneOf12>::arm_6(fetch::<C>(g)),
        7 => <C as OneOf12>::arm_7(fetch::<C>(h)),
        8 => <C as OneOf12>::arm_8(fetch::<C>(i)),
        9 => <C as OneOf12>::arm_9(fetch::<C>(j)),
        10 => <C as OneOf12>::arm_10(fetch::<C>(k)),
        _ => <C as OneOf12>::arm_11(fetch::<C>(l)),
    })
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
