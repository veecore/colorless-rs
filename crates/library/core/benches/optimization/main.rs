//! Microbenchmarks for how well native optimization sees through Colorless.
//!
//! Keep native and Colorless cases structurally equivalent. Inputs and outputs
//! may be passed through `black_box`, but the implementation under test should
//! remain visible to the optimizer.

mod branch;
mod cps;
mod loops;
mod one_of_12;
mod one_of_4;

use criterion::{Criterion, criterion_group, criterion_main};

criterion_group! {
    name = optimization;
    config = Criterion::default();
    targets =
        cps::bench_sync_cps,
        cps::bench_async_cps,
        branch::bench_sync_branch,
        branch::bench_nested_sync_branch,
        branch::bench_async_branch,
        branch::bench_nested_async_branch,
        one_of_4::bench_sync_match,
        one_of_4::bench_sync_match_const_arms,
        one_of_4::bench_async_match,
        one_of_4::bench_async_match_const_arms,
        one_of_12::bench_sync_match,
        one_of_12::bench_async_match,
        loops::bench_sync_loop,
        loops::bench_async_loop,
}

criterion_main!(optimization);
