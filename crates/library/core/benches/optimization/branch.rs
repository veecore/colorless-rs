use colorless_core::{
    AsyncExt, Color, Colored, IdentityExt, OneOf2, OneOf3, test_util::block_on_test,
};
use criterion::Criterion;
use std::hint::black_box;

// ── Native branch ─────────────────────────────────────────────────────────────
// Sync
#[inline(never)]
fn native_sync_branch(flag: bool, a: i32, b: i32) -> i32 {
    if flag { a * 10 } else { b * 100 }
}

#[inline(never)]
fn native_nested_sync_branch(outer: bool, inner: bool, a: i32, b: i32, c: i32) -> i32 {
    if outer {
        if inner { a * 10 } else { b * 10 }
    } else {
        c * 10
    }
}

// Async
#[inline(never)]
async fn native_leaf(v: i32) -> i32 {
    v * 10
}

#[inline(never)]
async fn native_async_branch(flag: bool, a: i32, b: i32) -> i32 {
    if flag {
        native_leaf(a).await
    } else {
        native_leaf(b).await
    }
}

#[inline(never)]
async fn native_nested_async_branch(outer: bool, inner: bool, a: i32, b: i32, c: i32) -> i32 {
    if outer {
        if inner {
            native_leaf(a).await
        } else {
            native_leaf(b).await
        }
    } else {
        native_leaf(c).await
    }
}

// ── Colored branch ───────────────

#[inline(never)]
fn fetch<C: Color>(v: i32) -> impl Colored<C, Output = i32> {
    C::ready(v * 10)
}

#[inline(never)]
fn colored_branch<C: Color>(flag: bool, a: i32, b: i32) -> impl Colored<C, Output = i32> {
    C::wrap_flatten(move || {
        if flag {
            <C as OneOf2>::arm_0(fetch(a))
        } else {
            <C as OneOf2>::arm_1(fetch(b))
        }
    })
}

//  FIXME: THIS GUY NOW TAKES 7NS WHILE NATIVE ASYNC 4NS...
//   THIS IS TROUBLING
#[inline(never)]
fn colored_nested_branch<C: Color>(
    outer: bool,
    inner: bool,
    a: i32,
    b: i32,
    c: i32,
) -> impl Colored<C, Output = i32> {
    C::wrap_flatten(move || {
        if outer {
            <C as OneOf2>::arm_0(if inner {
                <C as OneOf2>::arm_0(fetch(a))
            } else {
                <C as OneOf2>::arm_1(fetch(b))
            })
        } else {
            <C as OneOf2>::arm_1(fetch(c))
        }
    })
}

#[inline(never)]
fn colored_nested_branch_flattened<C: Color>(
    outer: bool,
    inner: bool,
    a: i32,
    b: i32,
    c: i32,
) -> impl Colored<C, Output = i32> {
    C::wrap_flatten(move || {
        if outer {
            if inner {
                <C as OneOf3>::arm_0(fetch(a))
            } else {
                <C as OneOf3>::arm_1(fetch(b))
            }
        } else {
            <C as OneOf3>::arm_2(fetch(c))
        }
    })
}

// ── Bench Sync ─────────────────────────────────────────────────────────────

pub(crate) fn bench_sync_branch(c: &mut Criterion) {
    let mut group = c.benchmark_group("sync_branch");

    group.bench_function("native_if_else", |b| {
        let mut flag = true;
        b.iter(|| {
            flag = !flag;
            native_sync_branch(black_box(flag), black_box(42), black_box(99))
        });
    });

    group.bench_function("colored_identity_branch", |b| {
        let mut flag = true;
        b.iter(|| {
            flag = !flag;
            colored_branch(black_box(flag), black_box(42), black_box(99)).sync_()
        });
    });

    group.finish();
}

pub(crate) fn bench_nested_sync_branch(c: &mut Criterion) {
    let mut group = c.benchmark_group("sync_nested_branch");

    group.bench_function("native_nested_if_else", |b| {
        let mut outer = true;
        let mut inner = true;
        b.iter(|| {
            outer = !outer;
            inner = !inner;
            native_nested_sync_branch(
                black_box(outer),
                black_box(inner),
                black_box(1),
                black_box(2),
                black_box(3),
            )
        });
    });

    group.bench_function("colored_nested_identity_branch", |b| {
        let mut outer = true;
        let mut inner = true;
        b.iter(|| {
            outer = !outer;
            inner = !inner;
            colored_nested_branch(
                black_box(outer),
                black_box(inner),
                black_box(1),
                black_box(2),
                black_box(3),
            )
            .sync_()
        });
    });

    group.finish();
}

// ── Bench Async ─────────────────────────────────────────────────────────────

pub(crate) fn bench_async_branch(c: &mut Criterion) {
    let mut group = c.benchmark_group("async_branch");

    group.bench_function("native_async_if_else", |b| {
        let mut flag = true;
        b.iter(|| {
            flag = !flag;
            block_on_test(native_async_branch(
                black_box(flag),
                black_box(42),
                black_box(99),
            ))
        });
    });

    group.bench_function("colored_async_branch", |b| {
        let mut flag = true;
        b.iter(|| {
            flag = !flag;
            block_on_test(colored_branch(black_box(flag), black_box(42), black_box(99)).async_())
        });
    });
}

pub(crate) fn bench_nested_async_branch(c: &mut Criterion) {
    let mut group = c.benchmark_group("async_nested_branch");

    group.bench_function("native_nested_async_if_else", |b| {
        let mut outer = true;
        let mut inner = true;
        b.iter(|| {
            outer = !outer;
            inner = !inner;
            block_on_test(native_nested_async_branch(
                black_box(outer),
                black_box(inner),
                black_box(1),
                black_box(2),
                black_box(3),
            ))
        });
    });

    group.bench_function("colored_nested_async_branch", |b| {
        let mut outer = true;
        let mut inner = true;
        b.iter(|| {
            outer = !outer;
            inner = !inner;
            block_on_test(
                colored_nested_branch(
                    black_box(outer),
                    black_box(inner),
                    black_box(1),
                    black_box(2),
                    black_box(3),
                )
                .async_(),
            )
        });
    });

    group.bench_function("colored_nested_async_branch_flattened", |b| {
        let mut outer = true;
        let mut inner = true;
        b.iter(|| {
            outer = !outer;
            inner = !inner;
            block_on_test(
                colored_nested_branch_flattened(
                    black_box(outer),
                    black_box(inner),
                    black_box(1),
                    black_box(2),
                    black_box(3),
                )
                .async_(),
            )
        });
    });

    group.finish();
}
