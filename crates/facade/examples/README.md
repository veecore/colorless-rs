# Colorless Examples

Runnable examples demonstrating write-once, context-generic control flow across synchronous and asynchronous execution.

## Running Examples

All examples require the `--features relaxed-drop` flag:

```sh
# 1. Flagship: Streaming key-value parser with sync & async callbacks
cargo run -p colorless --example stream_parser --features relaxed-drop

# 2. Sequential pipeline
cargo run -p colorless --example simple_pipeline --features relaxed-drop

# 3. Conditional branching with fallback
cargo run -p colorless --example retry_with_fallback --features relaxed-drop
```

---

## Example Catalog

### 1. [`stream_parser.rs`](stream_parser.rs) ⭐
* **What it demonstrates**: A streaming line parser where both input chunks (`source`) and parsed records (`sink`) are callbacks driven by Colorless (`ColoredFnMut`).
* **Key takeaway**: Shows a **true source of color**! The parser is written once; in synchronous mode it is passed direct closures (`from_fn`), while in asynchronous mode it is passed async closures (`from_async_fn`) awaiting I/O. Demonstrates `pin!`, `borrow!`, and `reborrow!` across loop suspension points.

### 2. [`simple_pipeline.rs`](simple_pipeline.rs)
* **What it demonstrates**: Defining `#[colorless]` functions and calling them with `.sync_()` (direct synchronous execution) and `.async_()` (unboxed future).
* **Key takeaway**: Shows how `.resolve` creates suspension points that evaluate directly without allocations in sync mode, and await cooperatively in async mode.

### 3. [`retry_with_fallback.rs`](retry_with_fallback.rs)
* **What it demonstrates**: Branching (`if let Some(...) ... else ...`) across `.resolve` points.
* **Key takeaway**: Shows that Colorless preserves natural Rust control flow across multiple async/sync suspension points.
