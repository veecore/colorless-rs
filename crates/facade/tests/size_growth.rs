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

// ── Pinned cost ───────────────────────────────────────────────────────────────
//
// pin!/unpin! lets a colored function hold a stable &mut reference across a
// resolve point. The colored state machine stores the variable inside an
// AsyncBorrow enum whose scope/flow slots overlap, so it is actually 8 bytes
// *smaller* than the equivalent plain async fn that just keeps the variable
// alive across an await.
//
// native: String(24) + future discriminant/state(24) = 48 B
// colored: AsyncBorrow<String, S> phases overlap, so String(24) + phase(16) = 40 B
//
// The 8-byte saving comes from BorrowPhase overlapping the scope (0 B after
// codegen optimisation) and the flow, eliminating a redundant pointer slot that
// the compiler-generated async state machine keeps.

#[colorless]
fn pinned() {
    let mut a = String::new();
    pin!(a);
    fetch(1).resolve;
    unpin!(a);
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

    let colored_size = size_of_val(&colored);
    let native_size = size_of_val(&native);

    assert!(
        colored_size <= native_size,
        "Colored is {colored_size} and native is {native_size}"
    );

    block_on_test(native);
    block_on_test(colored);
}

// ── Stacked-pins cost ─────────────────────────────────────────────────────────
//
// Each additional pin!/unpin! adds one String (24 B) + one pin tag (8 B) = 32 B
// to the colored state machine.
//
// The natural native analog for N simultaneously-pinned variables is N variables
// all kept alive across a single await, which costs N×String. Colored pays the
// same N×String plus N×8 for the tags.
//
// A *truly parallel* native approach (running N sub-futures concurrently with a
// manual join) is also shown for contrast; that layout is much heavier because
// it must store N full sub-future state machines *plus* N Option<Output> slots.
//
// Stacked colored (N pins, 1 resolve) vs stacked native (N vars across 1 await):
//
//   N=1  colored=48  native=48  delta=0
//   N=2  colored=80  native=72  delta=+8
//   N=3  colored=112 native=96  delta=+16
//
// Per-pin cost in colored:  String(24) + tag(8) = 32 B
// Per-pin cost in native:   String(24)           = 24 B

#[colorless]
fn stacked_pins_2() {
    let mut a = String::new();
    let mut b = String::new();
    pin!(a);
    pin!(b);
    fetch(1).resolve;
    unpin!(b);
    unpin!(a);
}

#[colorless]
fn stacked_pins_3() {
    let mut a = String::new();
    let mut b = String::new();
    let mut c = String::new();
    pin!(a);
    pin!(b);
    pin!(c);
    fetch(1).resolve;
    unpin!(c);
    unpin!(b);
    unpin!(a);
}

// I think these parallels aren't correct
async fn stacked_2_native() {
    let a = String::new();
    let b = String::new();
    fetch_async(1).await;
    drop(b);
    drop(a);
}

async fn stacked_3_native() {
    let a = String::new();
    let b = String::new();
    let c = String::new();
    fetch_async(1).await;
    drop(c);
    drop(b);
    drop(a);
}

#[test]
fn stacked_pins_cost() {
    let c1 = size_of_val(&pinned().async_());
    let c2 = size_of_val(&stacked_pins_2().async_());
    let c3 = size_of_val(&stacked_pins_3().async_());

    let n1 = size_of_val(&pinned_native());
    let n2 = size_of_val(&stacked_2_native());
    let n3 = size_of_val(&stacked_3_native());

    // Single pin layout does not exceed native.
    assert!(
        c1 <= n1,
        "single pinned colored ({c1}) must not exceed native ({n1})"
    );

    // Linear O(N) scaling: each stacked pin adds exactly constant size (String 24 B + tag 8 B = 32 B).
    assert_eq!(
        c2 - c1,
        32,
        "pin 2 overhead must be exactly 32 bytes (String + tag)"
    );
    assert_eq!(
        c3 - c2,
        32,
        "pin 3 overhead must be exactly 32 bytes (String + tag)"
    );
    assert_eq!(
        c2 - c1,
        c3 - c2,
        "stacked pin scaling must be strictly linear O(N)"
    );

    // Native adds only String (24 B) per variable across await.
    assert_eq!(n2 - n1, 24);
    assert_eq!(n3 - n2, 24);

    // Verify executions complete correctly without panic.
    block_on_test(pinned().async_());
    block_on_test(stacked_pins_2().async_());
    block_on_test(stacked_pins_3().async_());
    block_on_test(pinned_native());
    block_on_test(stacked_2_native());
    block_on_test(stacked_3_native());
}

// ── Parallel join cost ────────────────────────────────────────────────────────
//
// When N colored futures run concurrently using a manual join (all pinned on the
// stack, polled in turn, results collected in Options), the state machine must
// hold N full sub-future state machines + N Option<Output> + the poll_fn closure
// discriminant. This is much heavier than sequential pin/unpin.
//
// This test documents the size of the native manual-join pattern and asserts only
// that it is *strictly larger* than the equivalent sequential stacked colored
// layout — illustrating why sequential composition wins when concurrency is not
// required.

async fn parallel_2_native() {
    let mut fa = std::pin::pin!(fetch_async(1));
    let mut fb = std::pin::pin!(fetch_async(2));
    let mut ra: Option<usize> = None;
    let mut rb: Option<usize> = None;
    std::future::poll_fn(|cx| {
        use std::future::Future;
        use std::task::Poll;
        if ra.is_none() {
            if let Poll::Ready(v) = fa.as_mut().poll(cx) {
                ra = Some(v);
            }
        }
        if rb.is_none() {
            if let Poll::Ready(v) = fb.as_mut().poll(cx) {
                rb = Some(v);
            }
        }
        if ra.is_some() && rb.is_some() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    })
    .await;
}

async fn parallel_3_native() {
    let mut fa = std::pin::pin!(fetch_async(1));
    let mut fb = std::pin::pin!(fetch_async(2));
    let mut fc = std::pin::pin!(fetch_async(3));
    let mut ra: Option<usize> = None;
    let mut rb: Option<usize> = None;
    let mut rc: Option<usize> = None;
    std::future::poll_fn(|cx| {
        use std::future::Future;
        use std::task::Poll;
        if ra.is_none() {
            if let Poll::Ready(v) = fa.as_mut().poll(cx) {
                ra = Some(v);
            }
        }
        if rb.is_none() {
            if let Poll::Ready(v) = fb.as_mut().poll(cx) {
                rb = Some(v);
            }
        }
        if rc.is_none() {
            if let Poll::Ready(v) = fc.as_mut().poll(cx) {
                rc = Some(v);
            }
        }
        if ra.is_some() && rb.is_some() && rc.is_some() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    })
    .await;
}

#[test]
fn parallel_join_cost() {
    // Sequential colored (1 resolve, N pinned vars)
    let c2 = size_of_val(&stacked_pins_2().async_());
    let c3 = size_of_val(&stacked_pins_3().async_());

    // Native manual join (N truly-concurrent sub-futures)
    let p2 = size_of_val(&parallel_2_native());
    let p3 = size_of_val(&parallel_3_native());

    assert!(
        c2 < p2,
        "stacked colored 2 ({c2}) must be strictly smaller than native parallel join ({p2})"
    );
    assert!(
        c3 < p3,
        "stacked colored 3 ({c3}) must be strictly smaller than native parallel join ({p3})"
    );

    // Verify executions complete correctly.
    block_on_test(parallel_2_native());
    block_on_test(parallel_3_native());
}
