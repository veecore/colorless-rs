# Colorless 🎨🚫

Colorless is a Rust toolkit for writing one body of control-flow that can run in multiple execution contexts, including direct synchronous code and unboxed asynchronous state machines. Its public facade is the `colorless` crate; `colorless-core` contains the runtime traits and built-in colors.

[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)
[![Rust](https://img.shields.io/badge/rust-2024%20(1.95+)-orange.svg)](Cargo.toml)
[![no_std](https://img.shields.io/badge/no__std-compatible-green.svg)](crates/library/core)

---

## The Dilemma: Function Coloring in Rust

In Rust, the ecosystem is fundamentally bifurcated into two worlds: **synchronous** (`std::io`, blocking loops, iterators) and **asynchronous** (`async`/`await`, `tokio`, futures).

If you are writing protocol parsers, serializers, streaming codecs, or core business logic, you are typically forced into a frustrating compromise:
1. **Duplicate your codebase**: Maintain and test nearly identical implementations (`parse_sync` vs `parse_async`).
2. **Accept runtime penalties**: Force users into `Box<dyn Future>`, dynamic trait objects, or runtime block-on wrappers that murder throughput and memory locality.

**Colorless provides a third way.**

By combining zero-cost algebraic runtime traits (`colorless-core`) with an ANF-based procedural lowering compiler (`colorless-macros`), Colorless allows you to write control-flow logic **once**. At compile time, the compiler monomorphizes your code directly into the target execution context:

* **Direct Synchronous Execution (`Identity`)**: Straight-line, native machine code with direct function calls and zero runtime abstraction penalty.
* **Unboxed Asynchronous Futures (`Async`)**: Tight, stack-allocated, unboxed asynchronous state machines without heap allocations.
* **Pluggable Colors**: Cleanly extensible to other execution semantics (fallibility, lazy evaluation, generators).

---

## Quick Tour

### 1. Basic Sequential Pipeline

```rust
use colorless::{colorless, AsyncExt, IdentityExt};

#[colorless]
fn step_a(x: i32) -> i32 {
    x + 10
}

#[colorless]
fn step_b(x: i32) -> i32 {
    x * 2
}

#[colorless]
fn calculate(value: i32) -> i32 {
    // `.resolve` marks a context-generic suspension point.
    // In synchronous mode, it invokes functions directly with zero overhead.
    // In asynchronous mode, it awaits cooperatively as an unboxed future.
    let a = step_a(value).resolve;
    let b = step_b(a).resolve;
    b - 5
}

// 1. Run synchronously — direct execution, zero overhead:
let sync_result = calculate(5).sync_();
assert_eq!(sync_result, 25);

// 2. Run asynchronously — unboxed future, zero heap allocations:
let async_future = calculate(5).async_();
// await in any async runtime (Tokio, async-std, smol, etc.)
```

### 2. Conditional Branching Across Suspension Points

```rust
use colorless::{colorless, AsyncExt, IdentityExt};

#[colorless]
fn primary_source(id: u32) -> Option<String> {
    if id == 42 { Some("Primary #42".into()) } else { None }
}

#[colorless]
fn fallback_source(id: u32) -> String {
    format!("Fallback cache #{id}")
}

#[colorless]
fn fetch_payload(id: u32) -> String {
    // Preserves standard Rust control flow across async/sync resolution:
    if let Some(data) = primary_source(id).resolve {
        data
    } else {
        fallback_source(id).resolve
    }
}

// Works identically in both worlds:
let sync_data = fetch_payload(42).sync_();
let async_data = fetch_payload(42).async_().await;
```

### 3. Streaming Parsers & Callbacks (True Source of Color)

```rust
use colorless::{colorless, AsyncExt, IdentityExt};
use colorless::function::{ColoredFnMut, from_fn, from_async_fn};

/// A streaming parser that works with sync or async sources and sinks.
#[colorless(C)]
fn parse_records<'a, S, D>(mut source: S, mut sink: D) -> Result<usize, ()>
where
    S: ColoredFnMut<(), C, Output = Option<&'a str>>,
    D: ColoredFnMut<(&'a str, &'a str), C, Output = Result<(), ()>>,
{
    let mut count = 0;
    // not std pin
    pin!(source);
    pin!(sink);

    #[state(mut source, mut sink, pub mut count)]
    while let Some(line) = rb!(source).call_mut(()).resolve {
        if let Some((key, val)) = line.split_once('=') {
            rb!(sink).call_mut((key.trim(), val.trim())).resolve?;
            count += 1;
        }
    }

    Ok(count)
}

// 1. Drive with synchronous closures pulling from in-memory iterators:
let sync_count = parse_records(
    from_fn(|| iter.next()),
    from_fn(|(k, v)| { println!("{k}={v}"); Ok(()) }),
).sync_()?;

// 2. Drive with asynchronous closures pulling from sockets or streams:
let async_count = parse_records(
    from_async_fn(async || socket.read_line().await),
    from_async_fn(async |(k, v)| { channel.send((k, v)).await; Ok(()) }),
).async_().await?;
```

---

## Key Highlights

* ⚡ **Zero-Cost & Unboxed**: No `Box<dyn Future>`, no dynamic dispatch, no hidden allocations. Generates monomorphized machine code or tightly-packed unboxed state machines.
* 📦 **`no_std` First-Class**: Core abstractions require no heap allocator—designed from day one for embedded systems, kernels, and memory-constrained environments.
* 🔄 **Procedural ANF Lowering**: The procedural macro transforms arbitrary high-level Rust syntax (branches, loops, early returns, match expressions) into an explicit, verifiable control-flow graph.
* 🛡️ **Inert Verification**: Emits parallel unreachable inert branches for the standard Rust borrow checker to validate lifetimes, moves, and borrow correctness before emitting runtime carriers.

---

## Status & Safety Model (Alpha)

This repository is preparing for its initial **alpha** release. The core runtime abstractions are stable and tested, but the procedural compiler is in active development and is not yet a transparent drop-in replacement for arbitrary Rust code.

### ⚠️ Relaxed Implicit Destruction Model

Macro-transformed code currently operates under a **relaxed implicit destruction model**:
* A local binding that crosses a `.resolve` point may be dropped earlier or later than the lexical end of its enclosing scope.
* Sibling locals may be dropped in an order different from standard Rust reverse-lexical drop order.

> [!IMPORTANT]
> **Do not use the macro on code whose correctness relies on precise RAII destruction timing** (e.g. mutex locks, database transaction guards, linear resources, or cleanup-sensitive subscriptions).  
> For cleanup-sensitive paths, use ordinary Rust or establish an explicit, separately-audited ownership boundary. This is a deliberate semantic trade-off of the current compiler model, not merely a temporary optimization quirk.

### Feature Maturity Matrix

| Feature | Status | Notes                                                             |
| :--- | :--- |:------------------------------------------------------------------|
| **Direct calls & `.resolve`** | ✅ Ready | Full support for chained, sequential, and nested resolve calls    |
| **Conditionals & Branching** | ✅ Ready | `if`/`else` and `match` expressions crossing resolve points       |
| **Loops** | 🔄 Guided | Supported via explicit `#[state(...)]` annotations                |
| **Local Pinning & Borrowing** | 🧪 Experimental | Handled via audited `colorless-rt` carriers; active research area |
| **Strict Lexical RAII** | 📋 Roadmap | Planned for the post-alpha production release                     |

---

## Examples

Runnable examples are located in [`crates/facade/examples/`](crates/facade/examples/):

```sh
# 1. Streaming parser with sync & async callbacks:
cargo run -p colorless --example stream_parser --features relaxed-drop

# 2. Sequential computation pipeline:
cargo run -p colorless --example simple_pipeline --features relaxed-drop

# 3. Conditional branching with fallback:
cargo run -p colorless --example retry_with_fallback --features relaxed-drop
```

See the [Examples Guide](crates/facade/examples/README.md) for more details.

---

## Repository Layout

* **`crates/facade` (`colorless`)**: The user-facing crate re-exporting core traits, runners (`sync_()`, `async_()`), and macros (`#[colorless]`).
* **`crates/library/core`**: `Color`, `Colored`, built-in execution contexts (`Identity`, `Async`), and zero-cost capability traits.
* **`crates/library/rt`**: Runtime support machinery, including audited local borrow carriers and safety guards.
* **`crates/compiler/{lower,codegen,macros}`**: The ANF lowering, carrier synthesis, and macro generation pipeline.

---

## Development & Verification

Verification and release gates are documented in [RELEASE.md](RELEASE.md). To run the full verification suite:

```sh
cargo fmt --all -- --check
cargo check --workspace --lib --bins --examples
cargo test --workspace
cargo test -p colorless --test macro_contract
cargo test -p colorless --test ui
```

* For the unsafe local-pinning contract and its test matrix, see [SAFETY.md](SAFETY.md).
* For dependency release order and crates.io publishing, see [PUBLISHING.md](PUBLISHING.md).
* Please read [CONTRIBUTING.md](CONTRIBUTING.md) before modifying lowering rules or runtime state machines.

---

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.
