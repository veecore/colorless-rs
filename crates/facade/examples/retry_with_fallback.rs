//! Demonstrates conditional branching across resolve points with Colorless.
//!
//! Run with:
//! ```sh
//! cargo run -p colorless --example retry_with_fallback --features relaxed-drop
//! ```

use colorless::{AsyncExt, IdentityExt, colorless, test_util::block_on_test};

#[colorless]
fn primary_data_source(id: u32) -> Option<String> {
    if id == 42 {
        Some("Primary payload for #42".into())
    } else {
        None
    }
}

#[colorless]
fn fallback_data_source(id: u32) -> String {
    format!("Fallback cache payload for #{id}")
}

#[colorless]
fn fetch_payload(id: u32) -> String {
    // Resolves primary source; if None, branches to resolve fallback source.
    if let Some(data) = primary_data_source(id).resolve {
        data
    } else {
        fallback_data_source(id).resolve
    }
}

fn main() {
    // 1. Synchronous execution:
    println!("--- Synchronous Execution ---");
    let primary_sync = fetch_payload(42).sync_();
    let fallback_sync = fetch_payload(99).sync_();
    println!("ID 42: {primary_sync}");
    println!("ID 99: {fallback_sync}");

    // 2. Asynchronous execution:
    println!("\n--- Asynchronous Execution ---");
    let primary_async = block_on_test(fetch_payload(42).async_());
    let fallback_async = block_on_test(fetch_payload(99).async_());
    println!("ID 42: {primary_async}");
    println!("ID 99: {fallback_async}");

    assert_eq!(primary_sync, primary_async);
    assert_eq!(fallback_sync, fallback_async);
    println!("\nAll fallback assertions passed!");
}
