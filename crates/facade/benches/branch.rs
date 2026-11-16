use colorless_core::{AsyncExt, IdentityExt, test_util::block_on_test};
use colorless_macros::colorless;
use criterion::{Criterion, criterion_group, criterion_main};
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

#[colorless]
#[inline(never)]
fn fetch(v: i32) -> i32 {
    v * 10
}

#[colorless]
#[inline(never)]
fn colored_branch(flag: bool, a: i32, b: i32) -> i32 {
    if flag {
        fetch(a).resolve
    } else {
        fetch(b).resolve
    }
}

#[colorless]
#[inline(never)]
fn colored_nested_branch(outer: bool, inner: bool, a: i32, b: i32, c: i32) -> i32 {
    if outer {
        if inner {
            fetch(a).resolve
        } else {
            fetch(b).resolve
        }
    } else {
        fetch(c).resolve
    }
}

#[colorless]
#[inline(never)]
fn colored_nested_branch_flattened(outer: bool, inner: bool, a: i32, b: i32, c: i32) -> i32 {
    if outer {
        if inner {
            fetch(a).resolve
        } else {
            fetch(b).resolve
        }
    } else {
        fetch(c).resolve
    }
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

criterion_group! {
    name = optimization;
    config = Criterion::default();
    targets =
        bench_sync_branch,
        bench_nested_sync_branch,
        bench_async_branch,
        bench_nested_async_branch,
}

criterion_main!(optimization);
