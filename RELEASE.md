# Release gate

The first public release should be an alpha.

## Required before any alpha publish

- [ ] `cargo fmt --all -- --check` succeeds.
- [ ] `cargo check --workspace --lib --bins --examples` succeeds on stable Rust.
- [ ] `cargo test --workspace --no-run` builds every test target on stable Rust.
- [ ] `cargo test --workspace` succeeds on stable Rust.
- [ ] Each opt-in nightly feature is checked with nightly Rust; it is not
  included in the stable all-features gate.
- [ ] `cargo +nightly miri test -p colorless-rt` succeeds for the local-pinning
  runtime (or a documented toolchain issue is resolved before release).
- [ ] `cargo test -p colorless --test ui` succeeds.
- [ ] No source-reachable compiler `panic!`, `todo!`, or `unimplemented!`
  remains; unsupported source receives a diagnostic.
- [ ] `MaybeUnpinned`, `ErasedBorrow`, and `LocalCell` have an explicit unsafe
  contract plus completion, cancellation, unwind, and exactly-once-drop
  tests.
- [ ] The public macro documentation names relaxed destruction timing and
  marks local pinning/automatic borrowing experimental.
- [ ] Every published package has a description, MSRV, README, and license
  metadata.
- [ ] Set the canonical `repository`, `homepage`, and `documentation` package
  metadata before publishing; see [PUBLISHING.md](PUBLISHING.md).
- [ ] Dry-run the versioned workspace packages in dependency order; see
  [PUBLISHING.md](PUBLISHING.md).
- [ ] Generated build artifacts and compiler ICE reports are absent from the
  release commit.

## Strict semantic mode, required for a production claim

- [ ] Preserve lexical drop scope and reverse sibling destruction order.
- [ ] Track binding identity through shadowing and branch joins.
- [ ] Model branch/loop state and abrupt exits rather than relying on closure
  capture layout.
- [ ] Support or explicitly reject every local-borrow and local-pin path join.

Until that second list is complete, relaxed destruction must be a conspicuous
acknowledgement in user-facing macro documentation.