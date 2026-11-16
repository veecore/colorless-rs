#![cfg(feature = "relaxed-drop")]
#![allow(clippy::let_and_return)]

use colorless::{AsyncExt, IdentityExt, colorless, test_util::block_on_local_test};
use colorless_core::ready;
use colorless_core::test_util::fetch;
use std::cell::Cell;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};

#[colorless]
fn relaxed_scope_mode_runs() -> usize {
    let value = {
        fetch(40).resolve;
        2
    };
    value
}

#[test]
fn relaxed_scope_mode_is_available_per_function() {
    assert_eq!(relaxed_scope_mode_runs().sync_(), 2);
    assert_eq!(block_on_local_test(relaxed_scope_mode_runs().async_()), 2);
}

// -----------------------------------------------------------------------------
// Labeled block returns value and drops locals by completion
// -----------------------------------------------------------------------------

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

#[colorless]
fn labeled_block_all_routes_exit(cond: bool) -> usize {
    let value = 'scope: {
        let heavy = heavy().resolve;
        if cond {
            let _keep_until_scope_exit = &heavy;
            break 'scope 1;
        } else {
            let _keep_until_scope_exit = &heavy;
            break 'scope 2;
        }
    };

    value
}

#[test]
fn a_labeled_block_returns_its_value_and_drops_by_completion() {
    for cond in [false, true] {
        DROPPED.store(false);
        assert_eq!(
            labeled_block_all_routes_exit(cond).sync_(),
            usize::from(!cond) + 1
        );
        assert!(DROPPED.load());

        DROPPED.store(false);
        assert_eq!(
            block_on_local_test(labeled_block_all_routes_exit(cond).async_()),
            usize::from(!cond) + 1
        );
        assert!(DROPPED.load());
    }
}

// -----------------------------------------------------------------------------
// Exits to different enclosing scope owners
// -----------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct DropLog(Arc<Mutex<Vec<&'static str>>>);

impl DropLog {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(Vec::new())))
    }

    fn entries(&self) -> Vec<&'static str> {
        self.0.lock().unwrap().clone()
    }

    fn assert_entries_unordered(&self, expected: &[&'static str]) {
        let mut actual = self.entries();
        actual.sort_unstable();
        let mut expected = expected.to_vec();
        expected.sort_unstable();
        assert_eq!(actual, expected);
    }
}

struct Probe {
    name: &'static str,
    log: DropLog,
}

impl Probe {
    fn new(name: &'static str, log: &DropLog) -> Self {
        Self {
            name,
            log: log.clone(),
        }
    }

    fn keep(&self) {
        std::hint::black_box(self);
    }
}

impl Drop for Probe {
    fn drop(&mut self) {
        self.log.0.lock().unwrap().push(self.name);
    }
}

#[colorless]
fn exits_to_different_scope_owners(cond: bool, log: DropLog) -> usize {
    let value = 'outer: {
        let outer = Probe::new("outer", &log);

        let inner_value = 'inner: {
            let inner = Probe::new("inner", &log);
            ready(()).resolve;
            inner.keep();

            if cond {
                break 'outer 10;
            } else {
                break 'inner 2;
            }
        };

        // Only the inner-exit route may execute this continuation, and it
        // must still be physically inside the outer scope.
        log.0.lock().unwrap().push("middle");
        assert_eq!(inner_value, 2);
        outer.keep();
        ready(()).resolve;
        break 'outer inner_value + 1;
    };

    value
}

#[test]
fn inner_exit_preserves_its_intermediate_continuation_in_identity() {
    let log = DropLog::new();
    assert_eq!(
        exits_to_different_scope_owners(false, log.clone()).sync_(),
        3
    );
    log.assert_entries_unordered(&["inner", "middle", "outer"]);
}

#[test]
fn outer_exit_skips_the_intermediate_continuation_in_identity() {
    let log = DropLog::new();
    assert_eq!(
        exits_to_different_scope_owners(true, log.clone()).sync_(),
        10
    );
    log.assert_entries_unordered(&["inner", "outer"]);
}

#[test]
fn inner_exit_preserves_its_intermediate_continuation_in_async() {
    let log = DropLog::new();
    assert_eq!(
        block_on_local_test(exits_to_different_scope_owners(false, log.clone()).async_()),
        3
    );
    log.assert_entries_unordered(&["inner", "middle", "outer"]);
}

#[test]
fn outer_exit_skips_the_intermediate_continuation_in_async() {
    let log = DropLog::new();
    assert_eq!(
        block_on_local_test(exits_to_different_scope_owners(true, log.clone()).async_()),
        10
    );
    log.assert_entries_unordered(&["inner", "outer"]);
}
