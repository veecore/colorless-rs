//! Representative workloads for costs experienced by Colorless users.
//!
//! Unlike the optimization suite, async cases run inside a Tokio current-thread
//! runtime and contain genuine pending-to-ready transitions. Executor entry is
//! not charged once per measured operation.

mod support;
mod suspending_loops;

use criterion::{Criterion, criterion_group, criterion_main};

criterion_group! {
    name = workloads;
    config = Criterion::default();
    targets =
        suspending_loops::bench_async_loop,
}

criterion_main!(workloads);
