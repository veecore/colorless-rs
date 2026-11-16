#![cfg(feature = "relaxed-drop")]
#![allow(clippy::never_loop)]

use colorless::{AsyncExt, IdentityExt, colorless, test_util::block_on_test};
use colorless_core::ready;
use std::cell::Cell;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};

thread_local! {
    static DROPPED_ON_THIS_THREAD: Cell<bool> = const { Cell::new(false) };
}

struct DropFlag;

impl DropFlag {
    fn load(&self) -> bool {
        DROPPED_ON_THIS_THREAD.get()
    }

    fn store(&self, value: bool) {
        DROPPED_ON_THIS_THREAD.set(value);
    }
}

static DROPPED: DropFlag = DropFlag;

#[colorless]
fn heavy() -> impl Drop + Debug {
    #[derive(Debug)]
    struct Me;

    impl Drop for Me {
        fn drop(&mut self) {
            DROPPED.store(true);
        }
    }
    Me
}

/// Loop congestion test:
/// Proves that temporaries created in each loop iteration drop before the next
/// iteration begins, preventing unbounded resource pile-up.
#[test]
fn loop_congestion_objects_drop_per_iteration() {
    #[colorless]
    fn run() {
        for iter in 0..4 {
            if iter != 0 {
                assert!(DROPPED.load());
                DROPPED.store(false);
            }
            let _heavy = heavy().resolve;
        }
    }

    DROPPED.store(false);
    run().sync_();

    DROPPED.store(false);
    block_on_test(run().async_());
}

/// Nested loops with labeled break to outer loop ('a)
#[test]
fn nested_loops_break_outer_drops_heavy_resource() {
    #[colorless]
    fn run() {
        'a: loop {
            let _h = heavy().resolve;
            loop {
                if true {
                    break 'a;
                }
            }
        }
    }

    DROPPED.store(false);
    run().sync_();
    assert!(DROPPED.load());

    DROPPED.store(false);
    block_on_test(run().async_());
    assert!(DROPPED.load());
}

/// While and while-let loops using `#[colorless::loop_state]`
#[test]
fn while_and_while_let_execution() {
    #[colorless]
    fn run(
        mut count: i32,
        abort_at: i32,
        mut items: Vec<Option<i32>>,
    ) -> Result<i32, &'static str> {
        let mut total = 0;

        #[colorless::loop_state(mut count, abort_at, pub mut total)]
        while count < 5 {
            ready(()).resolve;
            if count == abort_at {
                return Err("while count abort");
            }
            total += count * 10;
            count += 1;
        }

        #[colorless::loop_state(pub mut total, mut items)]
        while let Some(opt_val) = items.pop() {
            ready(()).resolve;
            let Some(val) = opt_val else {
                continue;
            };
            if val == -1 {
                return Err("while let negative abort");
            }
            total += val;
        }

        Ok(total)
    }

    let items = vec![Some(1), None, Some(2), Some(3)];
    let res_sync = run(0, -1, items.clone()).sync_();
    assert_eq!(res_sync, Ok(100 + 6));

    let res_async = block_on_test(run(0, -1, items).async_());
    assert_eq!(res_async, Ok(100 + 6));

    // Test abort in while
    let abort_while = run(0, 3, vec![]).sync_();
    assert_eq!(abort_while, Err("while count abort"));

    // Test abort in while-let
    let abort_while_let = run(5, -1, vec![Some(-1)]).sync_();
    assert_eq!(abort_while_let, Err("while let negative abort"));
}

/// Loop with reference across iterations
#[test]
fn loop_with_reference_drops_on_exit() {
    #[colorless]
    fn run() {
        loop {
            let h = heavy().resolve;
            #[colorless::loop_state(h)]
            loop {
                let _ref = &h;
                break;
            }
            break;
        }
    }

    DROPPED.store(false);
    run().sync_();
    assert!(DROPPED.load());

    DROPPED.store(false);
    block_on_test(run().async_());
    assert!(DROPPED.load());
}

/// Mutex guard dropped before resolve in branch, ensuring continuation doesn't deadlock
#[test]
fn mutex_released_before_resolve_test() {
    #[colorless]
    fn run(lock: Arc<Mutex<usize>>) -> usize {
        let b = if false {
            return 0;
        } else {
            let _v = lock.lock().unwrap();
            42
        };
        drop(
            lock.lock()
                .expect("lock must be released before continuation"),
        );
        ready(b).resolve
    }

    let lock = Arc::new(Mutex::new(0));
    assert_eq!(run(lock.clone()).sync_(), 42);
    assert_eq!(block_on_test(run(lock).async_()), 42);
}

/// Shadowed loop-state across nested loops with `#[inherit_no_shadow]`
#[test]
fn nested_loop_shadow_and_inherit_runs() {
    #[colorless]
    fn run() {
        let env = 4;
        #[colorless::loop_state(env)]
        loop {
            assert_eq!(env, 4);
            let env = String::from("bar");
            #[colorless::loop_state(env)]
            loop {
                assert_eq!(env, "bar");
                break;
            }
            break;
        }
    }

    run().sync_();
    block_on_test(run().async_());
}
