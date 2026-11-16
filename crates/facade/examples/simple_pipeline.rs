//! A simple example demonstrating write-once control flow with Colorless.
//!
//! Run with:
//! ```sh
//! cargo run -p colorless --example simple_pipeline --features relaxed-drop
//! ```

use colorless::{AsyncExt, IdentityExt, colorless, test_util::block_on_test};

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
    // `.resolve` marks context-generic suspension points.
    // In synchronous mode, it invokes functions directly with zero overhead.
    // In asynchronous mode, it awaits cooperatively as an unboxed future.
    let a = step_a(value).resolve;
    let b = step_b(a).resolve;
    b - 5
}

fn main() {
    // 1. Run synchronously:
    let sync_result = calculate(5).sync_();
    println!("Synchronous result:  {}", sync_result);
    assert_eq!(sync_result, (5 + 10) * 2 - 5); // 25

    // 2. Run asynchronously as an unboxed future:
    let async_future = calculate(5).async_();
    let async_result = block_on_test(async_future);
    println!("Asynchronous result: {}", async_result);
    assert_eq!(async_result, 25);

    println!("All pipeline assertions passed!");
}
