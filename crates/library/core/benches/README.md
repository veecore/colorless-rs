# Benchmark suites

The core benchmarks answer two separate questions and deliberately live in
separate binaries.

## `optimization`

These are small, paired native/Colorless kernels for determining how well
rustc can optimize through the core abstraction.

- Write the Colorless side directly against the current core traits.
- Give native and Colorless cases the same inputs, outputs, and suspension
  points.
- Put `black_box` around benchmark inputs and final results, not inside the
  implementation being optimized.
- Keep construction, polling, completion, and cancellation as separate cases
  when their costs need to be distinguished.
- Add depth variants when composition is under investigation.

Run the whole suite with:

```text
cargo bench -p colorless-core --bench optimization
```

Pass a Criterion filter to run a particular family:

```text
cargo bench -p colorless-core --bench optimization -- cps_async
```

## `workloads`

These are representative user-facing operations. Async cases run inside a
Tokio current-thread runtime and may include allocation, branching, borrowing,
loops, and repeated pending-to-ready transitions. Criterion enters the runtime
once per sample, rather than charging `Runtime::block_on` to every operation.

Construct runtimes and input fixtures outside timed iterations. Native and
Colorless implementations must still perform equivalent work.

If executor-entry overhead itself matters, benchmark it as a separate explicit
case with `Runtime::block_on` inside the timed iteration; do not mix that cost
into the ordinary future comparisons.

Run the suite with:

```text
cargo bench -p colorless-core --bench workloads
```

The erased async loop cases are available when the core's allocation support
is enabled:

```text
cargo bench -p colorless-core --features alloc --bench workloads
```

## Procedural macro cases

The optimization suite must not depend on procedural lowering: otherwise a
change in the macro and a change in the runtime become indistinguishable.

Until the current macro is ready, workloads use hand-written expansions of the
current design. Do not copy legacy expansions into the benchmark oracle. Once
the current frontend is usable, representative workloads can have three lanes:

1. native Rust;
2. hand-written core calls;
3. macro-generated Colorless code.

The hand-written lane then isolates runtime costs, while the difference between
the hand-written and generated lanes exposes lowering quality.

Future-size, drop-order, pinning, and panic-cleanup checks remain unit tests;
they are correctness invariants rather than timing benchmarks.
