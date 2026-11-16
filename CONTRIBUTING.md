# Contributing

Colorless has two correctness boundaries:

1. the runtime's unsafe state-machine and borrow-carrier implementations; and
2. the compiler's transformation from source Rust to those runtime operations.

Please keep them independently testable. Do not use a runtime assertion to
justify a compiler transformation that could have been rejected or represented
in typed IR.

## Before opening a change

Run the release commands in [RELEASE.md](RELEASE.md). Changes to generated
code also need at least one source-level UI test in `crates/facade/tests/ui`.
Changes touching `MaybeUnpinned`, `ErasedBorrow`, `LocalCell`, or an unsafe
`ColoredInternal` implementation need an executable safety regression test and
an updated rationale in [SAFETY.md](SAFETY.md).

## Compiler conventions

- Unsupported input must become `syn::Error`, never `panic!` or
  `unimplemented!` in a source-reachable path.
- Preserve the original source in inert validation whenever lowering introduces
  unsafe runtime support.
- A successful inert check is necessary evidence, not the whole unsafe proof:
  generated control flow, ownership transfer, and cancellation still need an
  explicit argument.
- Keep experimental features conservative. Rejecting an unproven shape is
  better than silently changing observable ownership behavior.

## Tests

- Unit tests live beside runtime and IR modules.
- `crates/facade/tests/ui/pass` holds supported macro examples.
- `crates/facade/tests/ui/fail` holds stable diagnostics. Update `.stderr`
  snapshots intentionally with `TRYBUILD=overwrite cargo test -p colorless --test ui`.
- `crates/facade/tests/macro_contract.rs` executes code produced by the public
  `#[colorless]` and `color!` macros in both Identity and Async contexts.
- End-to-end behavior tests belong in facade/codegen integration tests.

Benchmarks are diagnostic tools, not semantic evidence. Do not trade a safety
property for a benchmark result without making the contract explicit first.
