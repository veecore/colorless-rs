#![cfg(feature = "relaxed-drop")]
#![allow(clippy::let_and_return)]

use colorless::{AsyncExt, Color, Colored, IdentityExt, colorless, test_util::block_on_local_test};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

fn ready<C: Color, T>(value: T) -> impl Colored<C, Output = T> {
    C::ready(value)
}

#[colorless]
fn inner_owned_value_crosses_resolve() -> usize {
    let inner = String::from("inside");
    ready(()).resolve;
    inner.len()
}

#[test]
fn inner_owned_value_crosses_resolve_in_every_color() {
    assert_eq!(inner_owned_value_crosses_resolve().sync_(), 6);
    assert_eq!(
        block_on_local_test(inner_owned_value_crosses_resolve().async_()),
        6
    );
}

#[colorless]
fn borrowed_argument_crosses_resolve(value: &str) -> usize {
    ready(()).resolve;
    value.len()
}

#[test]
fn borrowed_argument_remains_owned_by_the_caller() {
    let value = String::from("outside");

    assert_eq!(borrowed_argument_crosses_resolve(&value).sync_(), 7);
    assert_eq!(value, "outside");

    assert_eq!(
        block_on_local_test(borrowed_argument_crosses_resolve(&value).async_()),
        7
    );
    assert_eq!(value, "outside");
}

#[colorless]
fn value_consumed_before_resolve(value: String) -> usize {
    let len = value.len();
    drop(value);
    ready(()).resolve;
    len
}

#[test]
fn a_value_consumed_before_resolve_is_not_captured_again() {
    assert_eq!(
        value_consumed_before_resolve(String::from("moved")).sync_(),
        5
    );
    assert_eq!(
        block_on_local_test(value_consumed_before_resolve(String::from("moved")).async_()),
        5
    );
}

struct DropFlag(Arc<AtomicBool>);

impl Drop for DropFlag {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

#[colorless]
fn drop_only_local_must_cross_resolve(dropped: Arc<AtomicBool>) {
    let _guard = DropFlag(dropped.clone());
    assert!(!dropped.load(Ordering::SeqCst));
    ready(()).resolve;
    assert!(!dropped.load(Ordering::SeqCst));
}

#[test]
fn drop_only_local_has_direct_style_timing_in_identity() {
    let dropped = Arc::new(AtomicBool::new(false));
    drop_only_local_must_cross_resolve(dropped.clone()).sync_();
    assert!(dropped.load(Ordering::SeqCst));
}

#[test]
#[ignore = "relaxed drop timing does not promise that an otherwise-unused local survives until its source scope ends"]
fn drop_only_local_has_direct_style_timing_in_async() {
    let dropped = Arc::new(AtomicBool::new(false));
    block_on_local_test(drop_only_local_must_cross_resolve(dropped.clone()).async_());
    assert!(dropped.load(Ordering::SeqCst));
}

#[colorless]
fn drop_only_local_is_dropped_by_completion(dropped: Arc<AtomicBool>) {
    let _guard = DropFlag(dropped.clone());
    ready(()).resolve;
}

#[test]
fn drop_only_local_is_gone_by_the_function_boundary_in_every_color() {
    let dropped = Arc::new(AtomicBool::new(false));
    drop_only_local_is_dropped_by_completion(dropped.clone()).sync_();
    assert!(dropped.load(Ordering::SeqCst));

    let dropped = Arc::new(AtomicBool::new(false));
    block_on_local_test(drop_only_local_is_dropped_by_completion(dropped.clone()).async_());
    assert!(dropped.load(Ordering::SeqCst));
}

#[derive(Clone)]
struct DropLog(Arc<Mutex<Vec<&'static str>>>);

impl DropLog {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(Vec::new())))
    }

    fn record(&self, name: &'static str) {
        self.0.lock().unwrap().push(name);
    }

    fn assert_eq(&self, expected: &[&'static str]) {
        let actual = self.0.lock().unwrap().clone();
        assert_eq!(actual, expected);
    }

    fn assert_unordered(&self, expected: &[&'static str]) {
        let mut actual = self.0.lock().unwrap().clone();
        actual.sort_unstable();
        let mut expected = expected.to_vec();
        expected.sort_unstable();
        assert_eq!(actual, expected);
    }
}

struct DropProbe {
    name: &'static str,
    log: DropLog,
}

impl DropProbe {
    fn new(name: &'static str, log: &DropLog) -> Self {
        Self {
            name,
            log: log.clone(),
        }
    }

    fn name(&self) -> &'static str {
        self.name
    }
}

impl Drop for DropProbe {
    fn drop(&mut self) {
        self.log.record(self.name);
    }
}

#[colorless]
fn block_initializer_is_dropped_by_completion(log: DropLog) -> usize {
    let value = {
        let guard = DropProbe::new("initializer", &log);
        ready(()).resolve;
        log.assert_eq(&[]);
        guard.name().len()
    };

    ready(value).resolve
}

#[test]
fn block_initializer_is_dropped_by_completion_in_every_color() {
    let log = DropLog::new();
    assert_eq!(
        block_initializer_is_dropped_by_completion(log.clone()).sync_(),
        11
    );
    log.assert_eq(&["initializer"]);

    let log = DropLog::new();
    assert_eq!(
        block_on_local_test(block_initializer_is_dropped_by_completion(log.clone()).async_()),
        11
    );
    log.assert_eq(&["initializer"]);
}

#[colorless]
fn nested_blocks_drop_inside_out(log: DropLog) {
    {
        let outer = DropProbe::new("outer", &log);
        ready(()).resolve;
        log.assert_eq(&[]);

        {
            let inner = DropProbe::new("inner", &log);
            ready(()).resolve;
            log.assert_eq(&[]);
            assert_eq!(inner.name(), "inner");
        }

        ready(()).resolve;
        assert_eq!(outer.name(), "outer");
    }
}

#[test]
fn nested_block_locals_are_dropped_by_completion_in_every_color() {
    let log = DropLog::new();
    nested_blocks_drop_inside_out(log.clone()).sync_();
    log.assert_unordered(&["inner", "outer"]);

    let log = DropLog::new();
    block_on_local_test(nested_blocks_drop_inside_out(log.clone()).async_());
    log.assert_unordered(&["inner", "outer"]);
}

#[colorless]
fn if_let_binding_is_dropped_by_completion(log: DropLog) -> usize {
    let value = if let Some(guard) = ready(Some(DropProbe::new("if-let", &log))).resolve {
        log.assert_eq(&[]);
        guard.name().len()
    } else {
        0
    };

    value
}

#[test]
fn if_let_arm_binding_is_dropped_by_completion_in_every_color() {
    let log = DropLog::new();
    assert_eq!(
        if_let_binding_is_dropped_by_completion(log.clone()).sync_(),
        6
    );
    log.assert_eq(&["if-let"]);

    let log = DropLog::new();
    assert_eq!(
        block_on_local_test(if_let_binding_is_dropped_by_completion(log.clone()).async_()),
        6
    );
    log.assert_eq(&["if-let"]);
}

#[colorless]
fn if_let_binding_crosses_an_inner_resolve(log: DropLog) -> usize {
    let value = if let Some(guard) = ready(Some(DropProbe::new("if-let", &log))).resolve {
        log.assert_eq(&[]);
        let value = ready(guard.name().len()).resolve;
        value
    } else {
        0
    };

    value
}

#[test]
fn if_let_arm_binding_crosses_an_inner_resolve_in_identity() {
    let log = DropLog::new();
    assert_eq!(
        if_let_binding_crosses_an_inner_resolve(log.clone()).sync_(),
        6
    );
    log.assert_eq(&["if-let"]);
}

#[test]
fn if_let_arm_binding_crosses_an_inner_resolve_in_async() {
    let log = DropLog::new();
    assert_eq!(
        block_on_local_test(if_let_binding_crosses_an_inner_resolve(log.clone()).async_()),
        6
    );
    log.assert_eq(&["if-let"]);
}

#[colorless]
fn explicitly_dropped_local_is_not_carried_across_resolve(log: DropLog) {
    let guard = DropProbe::new("explicit", &log);
    log.assert_eq(&[]);
    drop(guard);
    log.assert_eq(&["explicit"]);

    ready(()).resolve;
    log.assert_eq(&["explicit"]);
}

#[test]
fn explicit_drop_is_preserved_exactly_once_in_every_color() {
    let log = DropLog::new();
    explicitly_dropped_local_is_not_carried_across_resolve(log.clone()).sync_();
    log.assert_eq(&["explicit"]);

    let log = DropLog::new();
    block_on_local_test(
        explicitly_dropped_local_is_not_carried_across_resolve(log.clone()).async_(),
    );
    log.assert_eq(&["explicit"]);
}

#[colorless]
fn match_binding_is_dropped_by_completion(log: DropLog) -> usize {
    let value = match ready(Some(DropProbe::new("match", &log))).resolve {
        Some(guard) => {
            log.assert_eq(&[]);
            guard.name().len()
        }
        None => 0,
    };

    value
}

#[test]
fn match_arm_binding_is_dropped_by_completion_in_every_color() {
    let log = DropLog::new();
    assert_eq!(
        match_binding_is_dropped_by_completion(log.clone()).sync_(),
        5
    );
    log.assert_eq(&["match"]);

    let log = DropLog::new();
    assert_eq!(
        block_on_local_test(match_binding_is_dropped_by_completion(log.clone()).async_()),
        5
    );
    log.assert_eq(&["match"]);
}

#[colorless]
fn live_local_drops_before_early_or_fallthrough_completion(log: DropLog, early: bool) -> usize {
    let guard = DropProbe::new("function", &log);
    ready(()).resolve;
    log.assert_eq(&[]);

    if early {
        return guard.name().len();
    }

    guard.name().len()
}

#[test]
fn live_local_drops_before_function_completion_on_every_path() {
    for early in [false, true] {
        let log = DropLog::new();
        assert_eq!(
            live_local_drops_before_early_or_fallthrough_completion(log.clone(), early).sync_(),
            8
        );
        log.assert_eq(&["function"]);

        let log = DropLog::new();
        assert_eq!(
            block_on_local_test(
                live_local_drops_before_early_or_fallthrough_completion(log.clone(), early)
                    .async_(),
            ),
            8
        );
        log.assert_eq(&["function"]);
    }
}

#[colorless]
fn locals_in_one_scope_drop_in_reverse_declaration_order(log: DropLog) {
    let first = DropProbe::new("first", &log);
    let second = DropProbe::new("second", &log);

    ready(()).resolve;
    assert_eq!(first.name(), "first");
    assert_eq!(second.name(), "second");
}

#[test]
fn native_same_scope_locals_drop_in_reverse_declaration_order() {
    let log = DropLog::new();
    {
        let _first = DropProbe::new("first", &log);
        let _second = DropProbe::new("second", &log);
    }
    log.assert_eq(&["second", "first"]);
}

#[test]
#[ignore = "move-closure capture fields currently replace Rust local declaration drop order"]
fn same_scope_drop_order_is_preserved_in_identity() {
    let log = DropLog::new();
    locals_in_one_scope_drop_in_reverse_declaration_order(log.clone()).sync_();
    log.assert_eq(&["second", "first"]);
}

#[test]
#[ignore = "move-closure capture fields currently replace Rust local declaration drop order"]
fn same_scope_drop_order_is_preserved_in_async() {
    let log = DropLog::new();
    block_on_local_test(
        locals_in_one_scope_drop_in_reverse_declaration_order(log.clone()).async_(),
    );
    log.assert_eq(&["second", "first"]);
}

#[colorless]
fn shadowed_locals_keep_their_lexical_scope(log: DropLog) {
    let value = DropProbe::new("outer", &log);
    ready(()).resolve;
    assert_eq!(value.name(), "outer");

    {
        let value = DropProbe::new("inner", &log);
        ready(()).resolve;
        assert_eq!(value.name(), "inner");
    }

    log.assert_eq(&["inner"]);
    ready(()).resolve;
    assert_eq!(value.name(), "outer");
}

#[test]
fn native_shadowed_bindings_keep_their_lexical_scope() {
    let log = DropLog::new();
    {
        let value = DropProbe::new("outer", &log);
        {
            let value = DropProbe::new("inner", &log);
            assert_eq!(value.name(), "inner");
        }
        log.assert_eq(&["inner"]);
        assert_eq!(value.name(), "outer");
    }
    log.assert_eq(&["inner", "outer"]);
}

#[test]
#[ignore = "the generated continuation currently drops the shadowed outer binding too early"]
fn shadowed_bindings_recover_the_outer_binding_in_identity() {
    let log = DropLog::new();
    shadowed_locals_keep_their_lexical_scope(log.clone()).sync_();
    log.assert_eq(&["inner", "outer"]);
}

#[test]
#[ignore = "the generated continuation currently drops the shadowed outer binding too early"]
fn shadowed_bindings_recover_the_outer_binding_in_async() {
    let log = DropLog::new();
    block_on_local_test(shadowed_locals_keep_their_lexical_scope(log.clone()).async_());
    log.assert_eq(&["inner", "outer"]);
}

#[allow(clippy::needless_late_init)]
#[colorless]
fn uninitialized_binding_assigned_before_resolve() -> i32 {
    let x;
    x = 42;
    ready(()).resolve;
    x
}

#[test]
fn uninitialized_binding_assigned_before_resolve_works() {
    assert_eq!(uninitialized_binding_assigned_before_resolve().sync_(), 42);
    assert_eq!(
        block_on_local_test(uninitialized_binding_assigned_before_resolve().async_()),
        42
    );
}
