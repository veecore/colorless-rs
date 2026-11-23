#![cfg(feature = "relaxed-drop")]
#![allow(clippy::let_and_return)]

//! Tests for branch type inference and `unify_value`.
//!
//! # The Problem Solved by `unify_value`
//!
//! In direct-style Rust, an `if-else` or `match` expression unifies its arm types
//! naturally:
//!
//! ```rust,ignore
//! let opt = if cond { Some(42) } else { None };
//! opt.is_some()
//! ```
//!
//! Here, Arm 1 produces `Some(i32)` and Arm 2 produces `None: Option<?T>`. Rust's
//! type checker unifies `?T = i32`, so `opt.is_some()` compiles even though `.is_some()`
//! does not constrain `?T`.
//!
//! In the CPS transform, however, each arm independently invokes the generated
//! continuation macro (e.g. `__continuation!(None)` vs `__continuation!(Some(42))`).
//! Without cross-arm unification, the `None` expansion would suffer from:
//! `cannot infer type of the type parameter T declared on the enum Option`.
//!
//! To solve this, Colorless codegen emits:
//!
//! ```rust,ignore
//! let __opt_ty = ::core::marker::PhantomData;
//! // in every arm's continuation entry:
//! let opt = ::colorless_rt::unify_value($opt, __opt_ty);
//! ```
//!
//! Tying all arm values to the shared `PhantomData` variable forces rustc to solve
//! the unified type across all branches before executing the continuation.

use colorless::{AsyncExt, IdentityExt, colorless, test_util::block_on_local_test};
use colorless_core::ready;
// -----------------------------------------------------------------------------
// 1. `if-else` returning `Option<T>` with `None` where usage does not constrain `T`
// -----------------------------------------------------------------------------

#[colorless]
fn branch_option_is_some(cond: bool, value: i32) -> bool {
    let opt = if cond {
        ready(()).resolve;
        Some(value)
    } else {
        None
    };

    // `.is_some()` is defined for all `Option<T>` and does NOT constrain `T`.
    // Without `unify_value`, the `None` branch fails type inference.
    opt.is_some()
}

#[test]
fn branch_option_is_some_infers_inner_type() {
    assert!(branch_option_is_some(true, 42).sync_());
    assert!(!branch_option_is_some(false, 42).sync_());

    assert!(block_on_local_test(
        branch_option_is_some(true, 42).async_()
    ));
    assert!(!block_on_local_test(
        branch_option_is_some(false, 42).async_()
    ));
}

// -----------------------------------------------------------------------------
// 2. Multi-arm `match` with multiple `None` arms
// -----------------------------------------------------------------------------

#[colorless]
fn match_option_multi_arm(mode: u8, base: i32) -> i32 {
    let opt = match mode {
        0 => {
            let v = ready(base).resolve;
            Some(v * 2)
        }
        1 => Some(base + 10),
        2 => None,
        _ => None,
    };

    // `.map` and `.unwrap_or` require `T` to be unified across all arms.
    opt.map(|x| x + 1).unwrap_or(-1)
}

#[test]
fn match_option_multi_arm_infers_across_all_arms() {
    assert_eq!(match_option_multi_arm(0, 5).sync_(), 11);
    assert_eq!(match_option_multi_arm(1, 5).sync_(), 16);
    assert_eq!(match_option_multi_arm(2, 5).sync_(), -1);
    assert_eq!(match_option_multi_arm(99, 5).sync_(), -1);

    assert_eq!(
        block_on_local_test(match_option_multi_arm(0, 5).async_()),
        11
    );
    assert_eq!(
        block_on_local_test(match_option_multi_arm(1, 5).async_()),
        16
    );
    assert_eq!(
        block_on_local_test(match_option_multi_arm(2, 5).async_()),
        -1
    );
    assert_eq!(
        block_on_local_test(match_option_multi_arm(99, 5).async_()),
        -1
    );
}

// -----------------------------------------------------------------------------
// 3. `Result<T, E>` where `Err` does not constrain `T`
// -----------------------------------------------------------------------------

#[colorless]
fn branch_result_is_ok(cond: bool, value: String) -> bool {
    let res: Result<String, &'static str> = if cond {
        let v = ready(value).resolve;
        Ok(v)
    } else {
        Err("failed")
    };

    // `.is_ok()` does not constrain `E` or `T` on either arm independently.
    res.is_ok()
}

#[colorless]
fn branch_result_inferred(cond: bool, value: String) -> bool {
    // Return type unannotated on `res` binding:
    let res = if cond {
        let v = ready(value).resolve;
        Ok(v)
    } else {
        Err("failed")
    };

    res.is_err()
}

#[test]
fn branch_result_infers_ok_and_err_types() {
    assert!(branch_result_is_ok(true, String::from("hello")).sync_());
    assert!(!branch_result_is_ok(false, String::from("hello")).sync_());

    assert!(!branch_result_inferred(true, String::from("hello")).sync_());
    assert!(branch_result_inferred(false, String::from("hello")).sync_());

    assert!(block_on_local_test(
        branch_result_is_ok(true, String::from("hello")).async_()
    ));
    assert!(!block_on_local_test(
        branch_result_is_ok(false, String::from("hello")).async_()
    ));
}

// -----------------------------------------------------------------------------
// 4. `Vec<T>` where `Vec::new()` arm does not constrain `T`
// -----------------------------------------------------------------------------

#[colorless]
fn branch_vec_empty(cond: bool, item: u64) -> usize {
    let list = if cond {
        let v = ready(item).resolve;
        vec![v, v + 1]
    } else {
        Vec::new()
    };

    // `.is_empty()` and `.len()` do NOT constrain `T`.
    if list.is_empty() { 0 } else { list.len() }
}

#[test]
fn branch_vec_empty_infers_element_type() {
    assert_eq!(branch_vec_empty(true, 10).sync_(), 2);
    assert_eq!(branch_vec_empty(false, 10).sync_(), 0);

    assert_eq!(block_on_local_test(branch_vec_empty(true, 10).async_()), 2);
    assert_eq!(block_on_local_test(branch_vec_empty(false, 10).async_()), 0);
}

// -----------------------------------------------------------------------------
// 5. Nested branches returning `Option<T>` with cascaded `None`
// -----------------------------------------------------------------------------

#[colorless]
fn nested_branch_option(outer: bool, inner: bool, value: i32) -> bool {
    let opt = if outer {
        if inner {
            Some(ready(value).resolve)
        } else {
            None
        }
    } else {
        None
    };

    opt.is_none()
}

#[test]
fn nested_branch_option_unifies_across_nesting_levels() {
    assert!(!nested_branch_option(true, true, 42).sync_());
    assert!(nested_branch_option(true, false, 42).sync_());
    assert!(nested_branch_option(false, true, 42).sync_());
    assert!(nested_branch_option(false, false, 42).sync_());

    assert!(!block_on_local_test(
        nested_branch_option(true, true, 42).async_()
    ));
    assert!(block_on_local_test(
        nested_branch_option(true, false, 42).async_()
    ));
    assert!(block_on_local_test(
        nested_branch_option(false, true, 42).async_()
    ));
    assert!(block_on_local_test(
        nested_branch_option(false, false, 42).async_()
    ));
}

// -----------------------------------------------------------------------------
// 6. Branch returning custom generic wrapper containing `Option<T>`
// -----------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
struct Wrapper<T>(Option<T>);

#[colorless]
fn branch_custom_generic_wrapper(cond: bool, value: usize) -> bool {
    let w = if cond {
        let v = ready(value).resolve;
        Wrapper(Some(v))
    } else {
        Wrapper(None)
    };

    w.0.is_some()
}

#[test]
fn branch_custom_generic_wrapper_infers_type() {
    assert!(branch_custom_generic_wrapper(true, 99).sync_());
    assert!(!branch_custom_generic_wrapper(false, 99).sync_());

    assert!(block_on_local_test(
        branch_custom_generic_wrapper(true, 99).async_()
    ));
    assert!(!block_on_local_test(
        branch_custom_generic_wrapper(false, 99).async_()
    ));
}

// -----------------------------------------------------------------------------
// 7. Branch returning `Default::default()` unified with concrete type
// -----------------------------------------------------------------------------

#[colorless]
fn branch_default_unification(cond: bool, input: String) -> usize {
    let value: String = if cond {
        let s = ready(input).resolve;
        s + "!"
    } else {
        Default::default()
    };

    value.len()
}

#[test]
fn branch_default_unification_infers_type() {
    assert_eq!(
        branch_default_unification(true, String::from("hi")).sync_(),
        3
    );
    assert_eq!(
        branch_default_unification(false, String::from("hi")).sync_(),
        0
    );

    assert_eq!(
        block_on_local_test(branch_default_unification(true, String::from("hi")).async_()),
        3
    );
    assert_eq!(
        block_on_local_test(branch_default_unification(false, String::from("hi")).async_()),
        0
    );
}

// -----------------------------------------------------------------------------
// 8. Branch inside loop returning `Option<usize>`
// -----------------------------------------------------------------------------

#[colorless]
fn branch_option_inside_loop(limit: usize) -> usize {
    let mut count = 0;
    let mut total = 0;

    #[colorless::state(limit, mut count, pub mut total)]
    while count < limit {
        let opt = if count % 2 == 0 {
            let v = ready(count).resolve;
            Some(v)
        } else {
            None
        };

        if let Some(val) = opt {
            total += val;
        }

        count += 1;
    }

    total
}

#[test]
fn branch_option_inside_loop_infers_and_executes() {
    // Evens in 0..6: 0 + 2 + 4 = 6
    assert_eq!(branch_option_inside_loop(6).sync_(), 6);
    assert_eq!(
        block_on_local_test(branch_option_inside_loop(6).async_()),
        6
    );
}

// -----------------------------------------------------------------------------
// 9. Branch with early return inside arm returning `Option<i32>`
// -----------------------------------------------------------------------------

#[colorless]
fn branch_option_early_return(cond: bool, early: bool) -> Option<i32> {
    let opt = if cond {
        if early {
            return None;
        }
        Some(ready(42).resolve)
    } else {
        None
    };

    opt
}

#[test]
fn branch_option_early_return_infers_type() {
    assert_eq!(branch_option_early_return(true, false).sync_(), Some(42));
    assert_eq!(branch_option_early_return(true, true).sync_(), None);
    assert_eq!(branch_option_early_return(false, false).sync_(), None);

    assert_eq!(
        block_on_local_test(branch_option_early_return(true, false).async_()),
        Some(42)
    );
    assert_eq!(
        block_on_local_test(branch_option_early_return(true, true).async_()),
        None
    );
    assert_eq!(
        block_on_local_test(branch_option_early_return(false, false).async_()),
        None
    );
}

// -----------------------------------------------------------------------------
// 10. `while-let` finding first matching `Option` and breaking
// -----------------------------------------------------------------------------

#[colorless]
fn branch_option_while_let_break(mut items: Vec<i32>) -> Option<i32> {
    let mut result: Option<i32> = None;

    #[colorless::state(mut items, pub mut result)]
    while let Some(item) = items.pop() {
        let opt = if item > 0 {
            Some(ready(item).resolve)
        } else {
            None
        };

        if opt.is_some() {
            result = opt;
            break;
        }
    }

    result
}

#[test]
fn branch_option_while_let_break_infers_type() {
    // All negative: loop exhausts without breaking, returning initial None
    assert_eq!(
        branch_option_while_let_break(vec![-2, -1, -5]).sync_(),
        None
    );
    assert_eq!(
        block_on_local_test(branch_option_while_let_break(vec![-2, -1, -5]).async_()),
        None
    );

    // Pops -5 (no match), then pops 42 (matches, breaks with Some(42))
    assert_eq!(
        branch_option_while_let_break(vec![-2, -1, 42, -5]).sync_(),
        Some(42)
    );
    assert_eq!(
        block_on_local_test(branch_option_while_let_break(vec![-2, -1, 42, -5]).async_()),
        Some(42)
    );

    // Pops 42 on first iteration
    assert_eq!(
        branch_option_while_let_break(vec![-2, -1, 42]).sync_(),
        Some(42)
    );
    assert_eq!(
        block_on_local_test(branch_option_while_let_break(vec![-2, -1, 42]).async_()),
        Some(42)
    );
}
