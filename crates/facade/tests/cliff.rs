#![cfg(feature = "relaxed-drop")]

// We test if strategically using color!{} could conquer cliffs

use colorless_core::test_util::block_on_test;
use colorless_core::{AsyncExt, IdentityExt};
#[allow(unused_imports)]
use colorless_macros::{color, colorless};
#[allow(unused_imports)]
use std::ops::ControlFlow;

#[colorless]
fn fetch(p: usize) -> usize {
    p
}

async fn fetch_async(p: usize) -> usize {
    p
}

#[cfg(feature = "compiler-cliff-probe")]
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

    let value_4 = match (seed + 4) % 4 {
        0 => fetch(seed + 4).resolve,
        1 => seed * 3 + 4,
        2 => fetch(seed * 5 + 4).resolve,
        _ => return seed + 4,
    };

    let value_5 = match (seed + 5) % 4 {
        0 => fetch(seed + 5).resolve,
        1 => seed * 3 + 5,
        2 => fetch(seed * 5 + 5).resolve,
        _ => return seed + 5,
    };

    fetch(value_0 + value_1 + value_2 + value_3 + value_4 + value_5).resolve
}

#[cfg(not(feature = "compiler-cliff-probe"))]
#[colorless(C)]
fn sequential_matches(seed: usize) -> usize {
    let value_0 = color! {move
        #![colorless(color = C)]

        ControlFlow::Continue(match seed % 4 {
            0 => fetch(seed).resolve,
            1 => seed * 3,
            2 => fetch(seed * 5).resolve,
            _ => return ControlFlow::Break(seed),
        })
    }
    .resolve;

    // somehow the match is back
    let value_0 = match value_0 {
        ControlFlow::Break(value_0) => return value_0,
        ControlFlow::Continue(value_0) => value_0,
    };

    let value_1 = color! {move
        // I actually thought the context would supply C... this is tedious
        #![colorless(color = C)]

         ControlFlow::Continue(match (seed + 1) % 4 {
            0 => fetch(seed + 1).resolve,
            1 => seed * 3 + 1,
            2 => fetch(seed * 5 + 1).resolve,
            _ => return ControlFlow::Break(seed + 1),
        })
    }
    .resolve;

    let value_1 = match value_1 {
        ControlFlow::Break(value_1) => return value_1,
        ControlFlow::Continue(value_1) => value_1,
    };

    let value_2 = color! {move
        #![colorless(color = C)]

        ControlFlow::Continue(match (seed + 2) % 4 {
            0 => fetch(seed + 2).resolve,
            1 => seed * 3 + 2,
            2 => fetch(seed * 5 + 2).resolve,
            _ => return ControlFlow::Break(seed + 2),
        })
    }
    .resolve;

    let value_2 = match value_2 {
        ControlFlow::Break(value_2) => return value_2,
        ControlFlow::Continue(value_2) => value_2,
    };

    let value_3 = color! {move
        #![colorless(color = C)]

        ControlFlow::Continue(match (seed + 3) % 4 {
            0 => fetch(seed + 3).resolve,
            1 => seed * 3 + 3,
            2 => fetch(seed * 5 + 3).resolve,
            _ => return ControlFlow::Break(seed + 3),
        })
    }
    .resolve;

    let value_3 = match value_3 {
        ControlFlow::Break(value_3) => return value_3,
        ControlFlow::Continue(value_3) => value_3,
    };

    let value_4 = color! {move
        #![colorless(color = C)]

        ControlFlow::Continue(match (seed + 4) % 4 {
            0 => fetch(seed + 4).resolve,
            1 => seed * 3 + 4,
            2 => fetch(seed * 5 + 4).resolve,
            _ => return ControlFlow::Break(seed + 4),
        })
    }
    .resolve;

    let value_4 = match value_4 {
        ControlFlow::Break(value_4) => return value_4,
        ControlFlow::Continue(value_4) => value_4,
    };

    let value_5 = color! {move
        #![colorless(color = C)]

        ControlFlow::Continue(match (seed + 5) % 4 {
            0 => fetch(seed + 5).resolve,
            1 => seed * 3 + 5,
            2 => fetch(seed * 5 + 5).resolve,
            _ => return ControlFlow::Break(seed + 5),
        })
    }
    .resolve;

    let value_5 = match value_5 {
        ControlFlow::Break(value_5) => return value_5,
        ControlFlow::Continue(value_5) => value_5,
    };

    fetch(value_0 + value_1 + value_2 + value_3 + value_4 + value_5).resolve
}

#[test]
fn generated_program_runs() {
    for seed in 0..4 {
        let sync = sequential_matches(seed).sync_();
        let async_ = block_on_test(sequential_matches(seed).async_());
        assert_eq!(sync, async_);
    }
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

    let value_4 = match (seed + 4) % 4 {
        0 => fetch_async(seed + 4).await,
        1 => seed * 3 + 4,
        2 => fetch_async(seed * 5 + 4).await,
        _ => return seed + 4,
    };

    let value_5 = match (seed + 5) % 4 {
        0 => fetch_async(seed + 5).await,
        1 => seed * 3 + 5,
        2 => fetch_async(seed * 5 + 5).await,
        _ => return seed + 5,
    };

    fetch_async(value_0 + value_1 + value_2 + value_3 + value_4 + value_5).await
}

#[test]
fn size_impact() {
    let native = sequential_matches_native(2);
    let colored = sequential_matches(2).async_();

    let colored_size = size_of_val(&colored);
    let native_size = size_of_val(&native);
    // turned out to be way smaller
    assert!(
        colored_size <= native_size + 24,
        "Colored is {colored_size} and native is {native_size}"
    );

    let native = block_on_test(native);
    let colored = block_on_test(colored);

    assert_eq!(native, colored);
}
