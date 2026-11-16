#![cfg(feature = "relaxed-drop")]
#![allow(clippy::let_and_return, clippy::let_unit_value, clippy::never_loop)]

use colorless_core::AsyncExt;
use colorless_core::test_util::block_on_test;
use colorless_macros::colorless;

#[colorless]
fn fetch(p: usize) -> usize {
    p
}

async fn fetch_async(p: usize) -> usize {
    p
}

#[colorless]
fn sequential() -> usize {
    let flow = fetch(0_usize).resolve;
    let flow = fetch(flow + 1).resolve;
    let flow = fetch(flow + 1).resolve;
    let flow = fetch(flow + 1).resolve;
    let flow = fetch(flow + 1).resolve;
    let flow = fetch(flow + 1).resolve;
    let flow = fetch(flow + 1).resolve;
    let flow = fetch(flow + 1).resolve;
    let flow = fetch(flow + 1).resolve;
    let flow = fetch(flow + 1).resolve;
    let flow = fetch(flow + 1).resolve;
    let flow = fetch(flow + 1).resolve;
    let flow = fetch(flow + 1).resolve;
    let flow = fetch(flow + 1).resolve;
    let flow = fetch(flow + 1).resolve;
    flow
}

async fn sequential_native() -> usize {
    let flow = fetch_async(0_usize).await;
    let flow = fetch_async(flow + 1).await;
    let flow = fetch_async(flow + 1).await;
    let flow = fetch_async(flow + 1).await;
    let flow = fetch_async(flow + 1).await;
    let flow = fetch_async(flow + 1).await;
    let flow = fetch_async(flow + 1).await;
    let flow = fetch_async(flow + 1).await;
    let flow = fetch_async(flow + 1).await;
    let flow = fetch_async(flow + 1).await;
    let flow = fetch_async(flow + 1).await;
    let flow = fetch_async(flow + 1).await;
    let flow = fetch_async(flow + 1).await;
    let flow = fetch_async(flow + 1).await;
    let flow = fetch_async(flow + 1).await;
    flow
}

#[test]
fn sequential_test() {
    let native = sequential_native();
    let colored = sequential().async_();

    let colored_size = size_of_val(&colored);
    let native_size = size_of_val(&native);
    // native keeps more things alive than we do
    assert!(
        colored_size <= native_size,
        "Colored is {colored_size} and native is {native_size}"
    );

    let native = block_on_test(native);
    let colored = block_on_test(colored);

    assert_eq!(native, colored);
}

#[colorless]
fn deep_blocks() {
    let _a = {
        fetch(1).resolve;
        fetch(1).resolve;
        fetch(1).resolve;

        let _a = {
            fetch(1).resolve;
            fetch(1).resolve;
            fetch(1).resolve;
        };
    };

    let _a = {
        fetch(1).resolve;
        fetch(1).resolve;
        fetch(1).resolve;

        let _a = {
            fetch(1).resolve;
            fetch(1).resolve;
            fetch(1).resolve;
        };
    };

    let _a = {
        fetch(1).resolve;
        fetch(1).resolve;
        fetch(1).resolve;

        let _a = {
            fetch(1).resolve;
            fetch(1).resolve;
            fetch(1).resolve;
        };
    };

    let _a = {
        fetch(1).resolve;
        fetch(1).resolve;
        fetch(1).resolve;

        let _a = {
            fetch(1).resolve;
            fetch(1).resolve;
            fetch(1).resolve;
        };
    };

    let _a = {
        fetch(1).resolve;
        fetch(1).resolve;
        fetch(1).resolve;

        let _a = {
            fetch(1).resolve;
            fetch(1).resolve;
            fetch(1).resolve;
        };
    };
}

async fn deep_blocks_native_async() {
    let _a = {
        fetch_async(1).await;
        fetch_async(1).await;
        fetch_async(1).await;

        let _a = {
            fetch_async(1).await;
            fetch_async(1).await;
            fetch_async(1).await;
        };
    };

    let _a = {
        fetch_async(1).await;
        fetch_async(1).await;
        fetch_async(1).await;

        let _a = {
            fetch_async(1).await;
            fetch_async(1).await;
            fetch_async(1).await;
        };
    };

    let _a = {
        fetch_async(1).await;
        fetch_async(1).await;
        fetch_async(1).await;

        let _a = {
            fetch_async(1).await;
            fetch_async(1).await;
            fetch_async(1).await;
        };
    };

    let _a = {
        fetch_async(1).await;
        fetch_async(1).await;
        fetch_async(1).await;

        let _a = {
            fetch_async(1).await;
            fetch_async(1).await;
            fetch_async(1).await;
        };
    };

    let _a = {
        fetch_async(1).await;
        fetch_async(1).await;
        fetch_async(1).await;

        let _a = {
            fetch_async(1).await;
            fetch_async(1).await;
            fetch_async(1).await;
        };
    };
}

#[test]
fn deep_blocks_test() {
    let native = deep_blocks_native_async();
    let colored = deep_blocks().async_();

    let colored = size_of_val(&colored);
    let native = size_of_val(&native);
    // native keeps more things alive than we do
    assert!(
        colored <= native,
        "Colored is {colored} and native is {native}"
    );
}

#[colorless]
fn sequential_matches(seed: usize) -> usize {
    let value_0 = match seed % 4 {
        0 => fetch(seed).resolve,
        1 => seed * 3,
        2 => fetch(seed * 5).resolve,
        _ => return seed,
    };

    let value_1 = match (seed + 1) % 4 {
        0 => fetch(seed + 1).resolve,
        1 => seed * 3 + 1,
        2 => fetch(seed * 5 + 1).resolve,
        _ => return seed + 1,
    };

    let value_2 = match (seed + 2) % 4 {
        0 => fetch(seed + 2).resolve,
        1 => seed * 3 + 2,
        2 => fetch(seed * 5 + 2).resolve,
        _ => return seed + 2,
    };

    let value_3 = match (seed + 3) % 4 {
        0 => fetch(seed + 3).resolve,
        1 => seed * 3 + 3,
        2 => fetch(seed * 5 + 3).resolve,
        _ => return seed + 3,
    };

    fetch(value_0 + value_1 + value_2 + value_3).resolve
}

async fn sequential_matches_native(seed: usize) -> usize {
    let value_0 = match seed % 4 {
        0 => fetch_async(seed).await,
        1 => seed * 3,
        2 => fetch_async(seed * 5).await,
        _ => return seed,
    };

    let value_1 = match (seed + 1) % 4 {
        0 => fetch_async(seed + 1).await,
        1 => seed * 3 + 1,
        2 => fetch_async(seed * 5 + 1).await,
        _ => return seed + 1,
    };

    let value_2 = match (seed + 2) % 4 {
        0 => fetch_async(seed + 2).await,
        1 => seed * 3 + 2,
        2 => fetch_async(seed * 5 + 2).await,
        _ => return seed + 2,
    };

    let value_3 = match (seed + 3) % 4 {
        0 => fetch_async(seed + 3).await,
        1 => seed * 3 + 3,
        2 => fetch_async(seed * 5 + 3).await,
        _ => return seed + 3,
    };

    fetch_async(value_0 + value_1 + value_2 + value_3).await
}

#[test]
fn sequential_matches_test() {
    let native = sequential_matches_native(2);
    let colored = sequential_matches(2).async_();

    let colored_size = size_of_val(&colored);
    let native_size = size_of_val(&native);
    // yup... cracks.......
    assert_eq!(
        colored_size,
        native_size + 8,
        "Colored is {colored_size} and native is {native_size}"
    );

    let native = block_on_test(native);
    let colored = block_on_test(colored);

    assert_eq!(native, colored);
}

#[colorless]
fn deep_loops() {
    let _a = loop {
        fetch(1).resolve;
        fetch(1).resolve;
        fetch(1).resolve;

        let _a = loop {
            fetch(1).resolve;
            fetch(1).resolve;
            fetch(1).resolve;
            break;
        };
        break;
    };

    let _a = loop {
        fetch(1).resolve;
        fetch(1).resolve;
        fetch(1).resolve;

        let _a = loop {
            fetch(1).resolve;
            fetch(1).resolve;
            fetch(1).resolve;
            break;
        };
        break;
    };

    let _a = loop {
        fetch(1).resolve;
        fetch(1).resolve;
        fetch(1).resolve;

        let _a = loop {
            fetch(1).resolve;
            fetch(1).resolve;
            fetch(1).resolve;
            break;
        };
        break;
    };

    let _a = loop {
        fetch(1).resolve;
        fetch(1).resolve;
        fetch(1).resolve;

        let _a = loop {
            fetch(1).resolve;
            fetch(1).resolve;
            fetch(1).resolve;
            break;
        };
        break;
    };

    let _a = loop {
        fetch(1).resolve;
        fetch(1).resolve;
        fetch(1).resolve;

        let _a = loop {
            fetch(1).resolve;
            fetch(1).resolve;
            fetch(1).resolve;
            break;
        };
        break;
    };
}

async fn deep_loops_async() {
    let _a = loop {
        fetch_async(1).await;
        fetch_async(1).await;
        fetch_async(1).await;

        let _a = loop {
            fetch_async(1).await;
            fetch_async(1).await;
            fetch_async(1).await;
            break;
        };
        break;
    };

    let _a = loop {
        fetch_async(1).await;
        fetch_async(1).await;
        fetch_async(1).await;

        let _a = loop {
            fetch_async(1).await;
            fetch_async(1).await;
            fetch_async(1).await;
            break;
        };
        break;
    };

    let _a = loop {
        fetch_async(1).await;
        fetch_async(1).await;
        fetch_async(1).await;

        let _a = loop {
            fetch_async(1).await;
            fetch_async(1).await;
            fetch_async(1).await;
            break;
        };
        break;
    };

    let _a = loop {
        fetch_async(1).await;
        fetch_async(1).await;
        fetch_async(1).await;

        let _a = loop {
            fetch_async(1).await;
            fetch_async(1).await;
            fetch_async(1).await;
            break;
        };
        break;
    };

    let _a = loop {
        fetch_async(1).await;
        fetch_async(1).await;
        fetch_async(1).await;

        let _a = loop {
            fetch_async(1).await;
            fetch_async(1).await;
            fetch_async(1).await;
            break;
        };
        break;
    };
}

#[test]
fn deep_loops_test() {
    let native = deep_loops_async();
    let colored = deep_loops().async_();

    let colored = size_of_val(&colored);
    let native = size_of_val(&native);
    // with states would be interesting but loop is really trash right now
    assert!(
        colored <= native,
        "Colored is {colored} and native is {native}"
    );
}

#[colorless]
fn pinned() {
    let mut a = String::new();
    pin_local!(a);
    fetch(1).resolve;
    unpin_local!(a);
    drop(a)
}

async fn pinned_native() {
    let a = String::new();
    fetch_async(1).await;
    drop(a);
}

#[test]
fn pinned_cost() {
    let native = pinned_native();
    let colored = pinned().async_();

    let colored = size_of_val(&colored);
    let native = size_of_val(&native);

    // the tag for pin itself and the async block in borrow
    assert_eq!(
        colored,
        native + 16,
        "Colored is {colored} and native is {native}"
    );
}
