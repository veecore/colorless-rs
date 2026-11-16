#![cfg(feature = "relaxed-drop")]

use colorless::{AsyncExt, IdentityExt, colorless, test_util::block_on_local_test};
use colorless_core::ready;
use std::cell::RefCell;

#[allow(clippy::needless_else)]
#[allow(clippy::if_same_then_else)]
#[colorless]
fn nested_branch_before_resolve(outer: bool, inner: bool) {
    if outer {
        if inner {
        } else {
        }
        ready(1).resolve;
    } else {
    }
}

#[test]
fn nested_branch_stays_inside_every_enclosing_join() {
    for outer in [false, true] {
        for inner in [false, true] {
            nested_branch_before_resolve(outer, inner).sync_();
            block_on_local_test(nested_branch_before_resolve(outer, inner).async_());
        }
    }
}

// Three plain arms share one carrier slot. The two colored arms each need a
// distinct slot because their concrete computations have different types.
#[colorless]
fn five_arms_with_three_plain(n: u8) {
    match n {
        0 => {}
        1 => {
            ready(()).resolve;
        }
        2 => {}
        3 => {
            ready(()).resolve;
        }
        _ => {}
    }
}

#[test]
fn mixed_five_arm_match_runs_every_route() {
    for n in 0..5 {
        five_arms_with_three_plain(n).sync_();
        block_on_local_test(five_arms_with_three_plain(n).async_());
    }
}

struct Events(RefCell<Vec<&'static str>>);

impl Events {
    fn new() -> Self {
        Self(RefCell::new(Vec::new()))
    }

    fn assert_is(&self, expected: &[&'static str]) {
        assert_eq!(&*self.0.borrow(), expected);
    }
}

struct DropEvent<'a> {
    events: &'a Events,
}

impl Drop for DropEvent<'_> {
    fn drop(&mut self) {
        self.events.0.borrow_mut().push("drop branch local");
    }
}

#[colorless]
fn relaxed_branch_drops_its_local_by_completion(run_arm: bool, events: &Events) {
    if run_arm {
        let guard = DropEvent { events };
        ready(()).resolve;
        events.assert_is(&[]);
        let _keep_alive_to_arm_end = &guard;
    } else {
        events.0.borrow_mut().push("run other arm");
    }

    events.0.borrow_mut().push("run continuation");
}

#[test]
fn relaxed_arm_preserves_last_use_and_drops_by_completion() {
    for async_ in [false, true] {
        let events = Events::new();
        if async_ {
            block_on_local_test(
                relaxed_branch_drops_its_local_by_completion(true, &events).async_(),
            );
        } else {
            relaxed_branch_drops_its_local_by_completion(true, &events).sync_();
        }
        let observed = events.0.borrow();
        assert_eq!(observed.len(), 2);
        assert!(observed.contains(&"drop branch local"));
        assert!(observed.contains(&"run continuation"));
        drop(observed);

        let events = Events::new();
        if async_ {
            block_on_local_test(
                relaxed_branch_drops_its_local_by_completion(false, &events).async_(),
            );
        } else {
            relaxed_branch_drops_its_local_by_completion(false, &events).sync_();
        }
        events.assert_is(&["run other arm", "run continuation"]);
    }
}

// -----------------------------------------------------------------------------
// Large multi-arm match tree carrier folding (40 distinct arms)
// -----------------------------------------------------------------------------
// Exercises the compiler's carrier tree-folding logic when branches exceed
// the maximum direct OneOf arm limit (12), generating shallow carrier trees.

#[allow(dead_code)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum WideEnum {
    V1,
    V2,
    V3,
    V4,
    V5,
    V6,
    V7,
    V8,
    V9,
    V10,
    V11,
    V12,
    V13,
    V14,
    V15,
    V16,
    V17,
    V18,
    V19,
    V20,
    V21,
    V22,
    V23,
    V24,
    V25,
    V26,
    V27,
    V28,
    V29,
    V30,
    V31,
    V32,
    V33,
    V34,
    V35,
    V36,
    V37,
    V38,
    V39,
    V40,
}

#[colorless]
fn many_arms(val: WideEnum) -> i32 {
    match val {
        WideEnum::V1 => ready(1).resolve,
        WideEnum::V2 => ready(2).resolve,
        WideEnum::V3 => ready(3).resolve,
        WideEnum::V4 => ready(4).resolve,
        WideEnum::V5 => ready(5).resolve,
        WideEnum::V6 => ready(6).resolve,
        WideEnum::V7 => ready(7).resolve,
        WideEnum::V8 => ready(8).resolve,
        WideEnum::V9 => ready(9).resolve,
        WideEnum::V10 => ready(10).resolve,
        WideEnum::V11 => ready(11).resolve,
        WideEnum::V12 => ready(12).resolve,
        WideEnum::V13 => ready(13).resolve,
        WideEnum::V14 => ready(14).resolve,
        WideEnum::V15 => ready(15).resolve,
        WideEnum::V16 => ready(16).resolve,
        WideEnum::V17 => ready(17).resolve,
        WideEnum::V18 => ready(18).resolve,
        WideEnum::V19 => ready(19).resolve,
        WideEnum::V20 => ready(20).resolve,
        WideEnum::V21 => ready(21).resolve,
        WideEnum::V22 => ready(22).resolve,
        WideEnum::V23 => ready(23).resolve,
        WideEnum::V24 => ready(24).resolve,
        WideEnum::V25 => ready(25).resolve,
        WideEnum::V26 => ready(26).resolve,
        WideEnum::V27 => ready(27).resolve,
        WideEnum::V28 => ready(28).resolve,
        WideEnum::V29 => ready(29).resolve,
        WideEnum::V30 => ready(30).resolve,
        WideEnum::V31 => ready(31).resolve,
        WideEnum::V32 => ready(32).resolve,
        WideEnum::V33 => ready(33).resolve,
        WideEnum::V34 => ready(34).resolve,
        WideEnum::V35 => ready(35).resolve,
        WideEnum::V36 => ready(36).resolve,
        WideEnum::V37 => ready(37).resolve,
        WideEnum::V38 => ready(38).resolve,
        WideEnum::V39 => ready(39).resolve,
        WideEnum::V40 => ready(40).resolve,
    }
}

#[test]
fn many_arms_tree_carrier_folds_cleanly() {
    for variant in [
        WideEnum::V1,
        WideEnum::V7,
        WideEnum::V12,
        WideEnum::V13,
        WideEnum::V25,
        WideEnum::V39,
        WideEnum::V40,
    ] {
        let expected = match variant {
            WideEnum::V1 => 1,
            WideEnum::V7 => 7,
            WideEnum::V12 => 12,
            WideEnum::V13 => 13,
            WideEnum::V25 => 25,
            WideEnum::V39 => 39,
            WideEnum::V40 => 40,
            _ => unreachable!(),
        };
        assert_eq!(many_arms(variant).sync_(), expected);
        assert_eq!(block_on_local_test(many_arms(variant).async_()), expected);
    }
}
