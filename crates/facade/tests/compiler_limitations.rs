#![cfg(feature = "relaxed-drop")]

//! Executable and documented compiler limitations and known boundaries.
//!
//! This suite gathers known limitations, intentionally ignored tests,
//! and structural constraints of the Colorless CPS transform and code generator.

use colorless::{AsyncExt, IdentityExt, colorless, test_util::block_on_local_test};
use colorless_core::ready;
use std::sync::Arc;
use std::sync::Mutex;

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

// -----------------------------------------------------------------------------
// Limitation 1: Reassigning a variable over its own resolve
// -----------------------------------------------------------------------------
// Universal limitation: `env = ready(env).resolve` moves `env` into the callee
// while simultaneously keeping the assignment target `env` open across a CPS stage.
#[cfg(any())]
#[test]
fn assign_over_resolve_limitation() {
    #[colorless]
    fn run() {
        let mut env = String::new();
        env = ready(env).resolve;
    }

    run().sync_();
}

// -----------------------------------------------------------------------------
// Limitation 2: Move-closure capture fields replace local drop order
// -----------------------------------------------------------------------------
// In native Rust, locals in the same lexical scope drop in reverse declaration order.
// When variables cross a `.resolve`, they are packed into generated carrier/closure structs,
// whose drop order follows struct field layout rather than statement order.
#[colorless]
fn locals_in_one_scope_drop(log: DropLog) {
    let first = DropProbe::new("first", &log);
    let second = DropProbe::new("second", &log);

    ready(()).resolve;
    assert_eq!(first.name(), "first");
    assert_eq!(second.name(), "second");
}

#[test]
#[ignore = "move-closure capture fields currently replace Rust local declaration drop order"]
fn same_scope_drop_order_in_identity() {
    let log = DropLog::new();
    locals_in_one_scope_drop(log.clone()).sync_();
    log.assert_eq(&["second", "first"]);
}

#[test]
#[ignore = "move-closure capture fields currently replace Rust local declaration drop order"]
fn same_scope_drop_order_in_async() {
    let log = DropLog::new();
    block_on_local_test(locals_in_one_scope_drop(log.clone()).async_());
    log.assert_eq(&["second", "first"]);
}

// -----------------------------------------------------------------------------
// Limitation 3: Continuation drops shadowed outer binding too early
// -----------------------------------------------------------------------------
// When an outer binding is shadowed inside an inner block and referenced again
// after the inner block, the generated CPS continuation drops the outer binding
// upon entering the inner block rather than restoring it.
#[colorless]
fn shadowed_locals_scope(log: DropLog) {
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
#[ignore = "the generated continuation currently drops the shadowed outer binding too early"]
fn shadowed_bindings_recovered_in_identity() {
    let log = DropLog::new();
    shadowed_locals_scope(log.clone()).sync_();
    log.assert_eq(&["inner", "outer"]);
}

#[test]
#[ignore = "the generated continuation currently drops the shadowed outer binding too early"]
fn shadowed_bindings_recovered_in_async() {
    let log = DropLog::new();
    block_on_local_test(shadowed_locals_scope(log.clone()).async_());
    log.assert_eq(&["inner", "outer"]);
}

// -----------------------------------------------------------------------------
// Limitation 4: Uninitialized bindings across a yield point
// -----------------------------------------------------------------------------
// In direct-style Rust, `let x;` followed by later assignment `x = 1;` is valid
// through definite-assignment analysis.
//
// However, when a yield point (`.resolve`) intervenes between declaration and
// initialization:
//
// ```rust,ignore
// let x;
// fetch().resolve;
// x = 1;
// ```
//
// The CPS transform generates a continuation closure (or carrier state struct)
// at the yield point that captures all in-scope locals. In Rust, a closure cannot
// capture or assign to an uninitialized outer binding.

#[cfg(any())]
#[test]
fn uninitialized_binding_across_resolve_limitation() {
    #[colorless]
    fn run() -> i32 {
        let x;
        ready(()).resolve;
        x = 1;
        x
    }

    let _ = run();
}

#[cfg(any())]
#[test]
fn typed_uninitialized_binding_across_resolve_limitation() {
    #[colorless]
    fn run() -> i32 {
        let x: i32;
        ready(()).resolve;
        x = 1;
        x
    }

    let _ = run();
}

#[cfg(any())]
#[test]
fn branch_initialized_binding_across_resolve_limitation() {
    #[colorless]
    fn run(cond: bool) -> i32 {
        let x;
        ready(()).resolve;
        if cond {
            x = 1;
        } else {
            x = 2;
        }
        x
    }

    let _ = run(true);
}

// -----------------------------------------------------------------------------
// Limitation 5: Lower moves the continuation into the branch
// -----------------------------------------------------------------------------
#[cfg(any())]
#[colorless]
fn compile_fail() -> impl FnOnce() {
    colorless_core::ready(1).resolve;
    if true {}
    let a = 1;
    move || println!("{a}")
}

// -----------------------------------------------------------------------------
// Limitation 6: Items duplication
// -----------------------------------------------------------------------------
#[cfg(any())]
#[colorless]
fn local_function() {
    fetch(()).resolve;

    do_some_work();

    if false {}

    fn do_some_work() {}
}
