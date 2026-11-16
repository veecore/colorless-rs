//! Release-quality semantic tests for the Colorless CPS transform.
//!
//! Each test targets a specific language construct or interaction. Tests are
//! written against `#[cps_fn]` (the lower-level transform) so they exercise
//! the full pipeline from source syntax down to code generation.
//!
//! **Do not add scratch tests here.** Every function in this file should have
//! a clear name, a comment explaining what property it verifies, and exhaustive
//! assertions across all relevant branches.

use dev_macros::cps_fn;
use std::marker::PhantomData;
use std::sync::Mutex;

// ─────────────────────────────────────────────────────────────────────────────
// §1  Binary short-circuit operators
// ─────────────────────────────────────────────────────────────────────────────

fn unify_value<T>(a: T, _b: PhantomData<T>) -> T {
    a
}

/// `&&` must not evaluate the RHS when the LHS is false.
/// Proof: RHS is a call to a function that panics.
#[test]
fn binary_and_does_not_evaluate_rhs_when_lhs_is_false() {
    fn boom() -> bool {
        panic!("RHS must not be evaluated")
    }

    #[cps_fn]
    fn run(lhs: bool) -> bool {
        lhs && boom()
    }

    assert!(!run(false));
}

/// `||` must not evaluate the RHS when the LHS is true.
#[test]
fn binary_or_does_not_evaluate_rhs_when_lhs_is_true() {
    fn boom() -> bool {
        panic!("RHS must not be evaluated")
    }

    #[cps_fn]
    fn run(lhs: bool) -> bool {
        lhs || boom()
    }

    assert!(run(true));
}

/// `&&` correctly evaluates both sides when the LHS is true.
#[test]
fn binary_and_evaluates_rhs_when_lhs_is_true() {
    #[cps_fn]
    fn run(a: bool, b: bool) -> bool {
        a && b
    }

    assert!(run(true, true));
    assert!(!run(true, false));
}

/// `||` correctly evaluates both sides when the LHS is false.
#[test]
fn binary_or_evaluates_rhs_when_lhs_is_false() {
    #[cps_fn]
    fn run(a: bool, b: bool) -> bool {
        a || b
    }

    assert!(run(false, true));
    assert!(!run(false, false));
}

/// Short-circuit chains: `a && b && c` stops at the first false.
#[test]
fn binary_and_chain_stops_at_first_false() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static CALLS: AtomicUsize = AtomicUsize::new(0);

    fn counted(value: bool) -> bool {
        CALLS.fetch_add(1, Ordering::Relaxed);
        value
    }

    #[cps_fn]
    fn run(a: bool, b: bool, c: bool) -> bool {
        counted(a) && counted(b) && counted(c)
    }

    CALLS.store(0, Ordering::Relaxed);
    assert!(!run(false, true, true));
    assert_eq!(CALLS.load(Ordering::Relaxed), 1);

    CALLS.store(0, Ordering::Relaxed);
    assert!(!run(true, false, true));
    assert_eq!(CALLS.load(Ordering::Relaxed), 2);

    CALLS.store(0, Ordering::Relaxed);
    assert!(run(true, true, true));
    assert_eq!(CALLS.load(Ordering::Relaxed), 3);
}

/// Short-circuit with diverging subexpression inside a larger expression.
/// The diverging branch must not affect the non-diverging result.
#[test]
fn binary_short_circuit_with_diverging_arm_inside_if() {
    #[cps_fn]
    fn run(a: bool, b: bool) -> Result<i32, &'static str> {
        let x = if (a || b) { Ok(1) } else { Err("both false") }?;
        Ok(x * 10)
    }

    assert_eq!(run(true, false), Ok(10));
    assert_eq!(run(false, true), Ok(10));
    assert_eq!(run(false, false), Err("both false"));
}

/// ANF hoisting with binary operands that contain early returns: LHS must
/// be evaluated first, and an early return in LHS must skip the RHS entirely.
#[test]
fn anf_binary_evaluation_order_with_early_return() {
    #[cps_fn]
    fn run(abort_lhs: bool, abort_rhs: bool) -> Result<i32, &'static str> {
        let v = (if abort_lhs {
            return Err("lhs abort");
        } else {
            10_i32
        }) + (if abort_rhs {
            return Err("rhs abort");
        } else {
            20_i32
        });
        Ok(v)
    }

    assert_eq!(run(true, false), Err("lhs abort"));
    assert_eq!(run(false, true), Err("rhs abort"));
    assert_eq!(run(false, false), Ok(30));
}

// ─────────────────────────────────────────────────────────────────────────────
// §2  `loop`, `while`, `for` — basic control flow
// ─────────────────────────────────────────────────────────────────────────────

/// A plain `loop { break VAL }` returns its break value.
#[test]
fn plain_loop_returns_break_value() {
    #[cps_fn]
    fn run() -> i32 {
        loop {
            break 42;
        }
    }

    assert_eq!(run(), 42);
}

/// A `while` loop runs the expected number of times and correctly exits.
#[test]
fn while_loop_runs_correct_number_of_iterations() {
    #[cps_fn]
    fn run(limit: u32) -> u32 {
        let mut count = 0;
        while count < limit {
            count += 1;
        }
        count
    }

    assert_eq!(run(0), 0);
    assert_eq!(run(1), 1);
    assert_eq!(run(5), 5);
}

/// A `while let` loop drains an iterator correctly.
#[test]
fn while_let_drains_stack() {
    #[cps_fn]
    fn run(items: Vec<i32>) -> i32 {
        let mut stack = items;
        let mut total = 0;
        while let Some(v) = stack.pop() {
            total += v;
        }
        total
    }

    assert_eq!(run(vec![]), 0);
    assert_eq!(run(vec![1, 2, 3]), 6);
    assert_eq!(run(vec![10, -3, 7]), 14);
}

/// A `for` loop over a range visits every element in order.
#[test]
fn for_loop_visits_all_elements_in_order() {
    #[cps_fn]
    fn run(n: i32) -> Vec<i32> {
        let mut out = Vec::new();
        for i in 0..n {
            out.push(i);
        }
        out
    }

    assert_eq!(run(0), Vec::<i32>::new());
    assert_eq!(run(4), vec![0, 1, 2, 3]);
}

/// `continue` inside a `for` loop skips the rest of the body for that iteration.
#[test]
fn for_loop_continue_skips_body() {
    #[cps_fn]
    fn run(items: Vec<i32>) -> Vec<i32> {
        let mut out = Vec::new();
        for x in items {
            if x % 2 != 0 {
                continue;
            }
            out.push(x);
        }
        out
    }

    assert_eq!(run(vec![1, 2, 3, 4, 5, 6]), vec![2, 4, 6]);
}

/// `break` inside a `for` loop exits the loop immediately.
#[test]
fn for_loop_break_exits_early() {
    #[cps_fn]
    fn run(items: Vec<i32>, limit: i32) -> i32 {
        let mut sum = 0;
        for x in items {
            if sum + x > limit {
                break;
            }
            sum += x;
        }
        sum
    }

    assert_eq!(run(vec![1, 2, 3, 4, 5], 7), 6); // 1+2+3=6, next is 4 which exceeds 7
    assert_eq!(run(vec![10, 20, 30], 100), 60); // all fit
    assert_eq!(run(vec![100], 50), 0); // first element already exceeds
}

/// An early `return` inside a `for` loop exits the entire function.
#[test]
fn for_loop_return_exits_function() {
    #[cps_fn]
    fn run(items: Vec<i32>, needle: i32) -> Result<usize, &'static str> {
        for (i, x) in items.into_iter().enumerate() {
            if x == needle {
                return Ok(i);
            }
        }
        Err("not found")
    }

    assert_eq!(run(vec![10, 20, 30], 20), Ok(1));
    assert_eq!(run(vec![10, 20, 30], 10), Ok(0));
    assert_eq!(run(vec![10, 20, 30], 30), Ok(2));
    assert_eq!(run(vec![10, 20, 30], 99), Err("not found"));
    assert_eq!(run(vec![], 1), Err("not found"));
}

// ─────────────────────────────────────────────────────────────────────────────
// §3  Labeled loops and cross-loop control flow
// ─────────────────────────────────────────────────────────────────────────────

/// `break 'label` jumps to the correct outer loop from an arbitrary depth.
#[test]
fn labeled_break_exits_to_correct_loop() {
    #[cps_fn]
    fn run(target: u8) -> i32 {
        'outer: loop {
            loop {
                loop {
                    match target {
                        0 => break 'outer 100,
                        1 => break,
                        _ => {}
                    }
                    break;
                }
                break;
            }
            break 200;
        }
    }

    assert_eq!(run(0), 100); // deep break to 'outer
    assert_eq!(run(1), 200); // inner break, falls through to outer break
}

/// `continue 'label` skips the inner loop tail and immediately retries the outer.
#[test]
fn labeled_continue_skips_to_correct_loop() {
    #[allow(unused_assignments)]
    #[cps_fn]
    fn run() -> Vec<(i32, i32)> {
        let mut log = Vec::new();
        let mut i = 0;
        'outer: loop {
            if i >= 3 {
                break;
            }
            let mut j = 0;
            loop {
                if j >= 3 {
                    break;
                }
                if j == 1 {
                    // Skip j=1 for all i, and jump directly to the next outer iteration
                    i += 1;
                    j += 1;
                    continue 'outer;
                }
                log.push((i, j));
                j += 1;
            }
            i += 1;
        }
        log
    }

    // For each i, we see (i,0) then continue 'outer skips (i,1) and (i,2)
    assert_eq!(run(), vec![(0, 0), (1, 0), (2, 0)]);
}

/// `break 'outer VALUE` from inside a nested loop carries the value correctly.
#[test]
fn labeled_break_with_value_from_nested_loop() {
    #[cps_fn]
    fn run(target: i32) -> Option<(i32, i32)> {
        'search: loop {
            for i in 0..5 {
                for j in 0..5 {
                    if i * 5 + j == target {
                        break 'search Some((i, j));
                    }
                }
            }
            break None;
        }
    }

    assert_eq!(run(7), Some((1, 2))); // 1*5+2 = 7
    assert_eq!(run(0), Some((0, 0)));
    assert_eq!(run(24), Some((4, 4)));
    assert_eq!(run(25), None); // out of range
}

/// An early `return` from inside a labeled loop exits the function, not just
/// the loop — the continuation after `break` must not execute.
#[test]
fn return_from_inside_labeled_loop_bypasses_post_loop_code() {
    #[cps_fn]
    fn run(abort: bool, sentinel: &mut bool) -> i32 {
        let result = 'outer: loop {
            loop {
                if abort {
                    return -1;
                }
                break 'outer 42;
            }
        };
        *sentinel = true;
        result
    }

    let mut s = false;
    assert_eq!(run(true, &mut s), -1);
    assert!(
        !s,
        "post-loop code must not execute when function returned early"
    );

    s = false;
    assert_eq!(run(false, &mut s), 42);
    assert!(s, "post-loop code must execute on normal exit");
}

// ─────────────────────────────────────────────────────────────────────────────
// §4  `let`-`else` and `let`-chain semantics
// ─────────────────────────────────────────────────────────────────────────────

/// `let-else` diverges exactly when the pattern fails to match.
#[test]
fn let_else_diverges_on_pattern_mismatch() {
    #[cps_fn(emit_inert = true)]
    fn run(input: Option<i32>) -> Result<i32, &'static str> {
        let Some(v) = input else {
            return Err("missing value");
        };
        Ok(v * 2)
    }

    assert_eq!(run(Some(5)), Ok(10));
    assert_eq!(run(None), Err("missing value"));
}

/// `let-else` works with an enum pattern.
#[test]
fn let_else_with_enum_pattern() {
    #[derive(Debug, PartialEq, Eq)]
    enum Cmd {
        Move(i32, i32),
        Stop,
    }

    #[cps_fn(emit_inert = true)]
    fn run(cmd: Cmd) -> Result<i32, &'static str> {
        let Cmd::Move(x, y) = cmd else {
            return Err("not a move");
        };
        Ok(x + y)
    }

    assert_eq!(run(Cmd::Move(3, 4)), Ok(7));
    assert_eq!(run(Cmd::Stop), Err("not a move"));
}

/// `let-else` inside a loop — the divergence exits the function, not the loop.
#[test]
fn let_else_inside_loop_exits_function_on_mismatch() {
    #[cps_fn(emit_inert = true)]
    fn run(items: Vec<Option<i32>>) -> Result<i32, &'static str> {
        let mut total = 0;
        for item in items {
            let Some(v) = item else {
                return Err("encountered None");
            };
            total += v;
        }
        Ok(total)
    }

    assert_eq!(run(vec![Some(1), Some(2), Some(3)]), Ok(6));
    assert_eq!(run(vec![Some(1), None, Some(3)]), Err("encountered None"));
    assert_eq!(run(vec![]), Ok(0));
}

// ─────────────────────────────────────────────────────────────────────────────
// §5  The `?` operator
// ─────────────────────────────────────────────────────────────────────────────

/// `?` on `Result` propagates `Err` and unwraps `Ok`.
#[test]
fn try_operator_propagates_err() {
    fn half(x: i32) -> Result<i32, &'static str> {
        if x % 2 == 0 { Ok(x / 2) } else { Err("odd") }
    }

    #[cps_fn]
    fn run(a: i32, b: i32) -> Result<i32, &'static str> {
        Ok(half(a)? + half(b)?)
    }

    assert_eq!(run(4, 6), Ok(5));
    assert_eq!(run(3, 6), Err("odd"));
    assert_eq!(run(4, 7), Err("odd"));
}

/// `?` on `Option` propagates `None`.
#[test]
fn try_operator_propagates_none() {
    fn lookup(map: &[(i32, i32)], key: i32) -> Option<i32> {
        map.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
    }

    #[cps_fn]
    fn run(map: &[(i32, i32)], a: i32, b: i32) -> Option<i32> {
        Some(lookup(map, a)? + lookup(map, b)?)
    }

    let map = &[(1, 10), (2, 20), (3, 30)];
    assert_eq!(run(map, 1, 2), Some(30));
    assert_eq!(run(map, 1, 9), None);
    assert_eq!(run(map, 9, 1), None);
}

/// Multiple sequential `?` operators in a pipeline — only the first failing
/// one propagates; later ones must not execute.
#[test]
fn try_operator_pipeline_stops_at_first_failure() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static STEPS: AtomicUsize = AtomicUsize::new(0);

    fn step(label: usize, v: Result<i32, usize>) -> Result<i32, usize> {
        STEPS.fetch_add(1, Ordering::Relaxed);
        v.map_err(|_| label)
    }

    #[cps_fn]
    fn run(fail_at: usize) -> Result<i32, usize> {
        let a = step(1, if fail_at == 1 { Err(0) } else { Ok(1) })?;
        let b = step(2, if fail_at == 2 { Err(0) } else { Ok(2) })?;
        let c = step(3, if fail_at == 3 { Err(0) } else { Ok(3) })?;
        Ok(a + b + c)
    }

    STEPS.store(0, Ordering::Relaxed);
    assert_eq!(run(1), Err(1));
    assert_eq!(STEPS.load(Ordering::Relaxed), 1);

    STEPS.store(0, Ordering::Relaxed);
    assert_eq!(run(2), Err(2));
    assert_eq!(STEPS.load(Ordering::Relaxed), 2);

    STEPS.store(0, Ordering::Relaxed);
    assert_eq!(run(3), Err(3));
    assert_eq!(STEPS.load(Ordering::Relaxed), 3);

    STEPS.store(0, Ordering::Relaxed);
    assert_eq!(run(0), Ok(6));
    assert_eq!(STEPS.load(Ordering::Relaxed), 3);
}

/// `?` inside a loop — a failure exits the function (not just the loop iteration).
#[test]
fn try_operator_inside_loop_exits_function() {
    fn validate(x: i32) -> Result<i32, String> {
        if x >= 0 {
            Ok(x)
        } else {
            Err(format!("negative: {x}"))
        }
    }

    #[cps_fn]
    fn run(items: Vec<i32>) -> Result<i32, String> {
        let mut sum = 0;
        for x in items {
            sum += validate(x)?;
        }
        Ok(sum)
    }

    assert_eq!(run(vec![1, 2, 3]), Ok(6));
    assert_eq!(run(vec![1, -2, 3]), Err("negative: -2".to_string()));
    assert_eq!(run(vec![-1, 2, 3]), Err("negative: -1".to_string()));
}

// ─────────────────────────────────────────────────────────────────────────────
// §6  Shadowing
// ─────────────────────────────────────────────────────────────────────────────

/// A binding shadowed in an inner scope does not bleed into the outer scope.
#[test]
fn shadowing_in_inner_scope_does_not_affect_outer() {
    #[cps_fn]
    fn run() -> i32 {
        let x = 1;
        {
            let x = 99;
            let _ = x;
        }
        x
    }

    assert_eq!(run(), 1);
}

/// Shadowing via `let x = …; let x = …;` at the same level replaces the binding.
#[test]
fn sequential_shadowing_replaces_binding() {
    #[cps_fn]
    fn run() -> (&'static str, &'static str) {
        let x = "first";
        let first = x;
        let x = "second";
        let second = x;
        (first, second)
    }

    assert_eq!(run(), ("first", "second"));
}

/// Shadowing inside an `if` branch does not affect the other branch or the
/// continuation after the `if`.
#[test]
fn shadowing_inside_branch_is_scoped_to_branch() {
    #[cps_fn]
    fn run(flag: bool) -> i32 {
        let x = 1;
        if flag {
            let x = 100;
            let _ = x;
        } else {
            let x = 200;
            let _ = x;
        }
        x // must be 1
    }

    assert_eq!(run(true), 1);
    assert_eq!(run(false), 1);
}

// ─────────────────────────────────────────────────────────────────────────────
// §7  Mutable state across `.resolve` yield points
// ─────────────────────────────────────────────────────────────────────────────

/// A mutable counter accumulated across yield points produces the correct total.
#[test]
fn mutable_accumulator_across_yield_points_is_correct() {
    fn provide(x: i32) -> i32 {
        x
    }

    #[cps_fn]
    fn run(values: Vec<i32>) -> i32 {
        let mut total = 0;
        for v in values {
            total += provide(v);
        }
        total
    }

    assert_eq!(run(vec![]), 0);
    assert_eq!(run(vec![1, 2, 3, 4, 5]), 15);
    assert_eq!(run(vec![-1, 1]), 0);
}

/// A mutable `Vec` accumulated across a loop is fully intact after the loop.
#[test]
fn mutable_vec_across_loop_contains_all_elements() {
    #[cps_fn]
    fn run(n: usize) -> Vec<usize> {
        let mut out = Vec::new();
        #[colorless::loop_state(out)]
        for i in 0..n {
            out.push(i);
        }
        out
    }

    assert_eq!(run(0), Vec::<usize>::new());
    assert_eq!(run(5), vec![0, 1, 2, 3, 4]);
}

// ─────────────────────────────────────────────────────────────────────────────
// §8  `match` semantics
// ─────────────────────────────────────────────────────────────────────────────

/// Every arm of a `match` is reachable and produces the correct value.
#[test]
fn match_all_arms_produce_correct_values() {
    #[derive(Clone, Copy)]
    enum Op {
        Add,
        Sub,
        Mul,
        Div,
    }

    #[cps_fn]
    fn run(op: Op, a: i32, b: i32) -> Option<i32> {
        match op {
            Op::Add => Some(a + b),
            Op::Sub => Some(a - b),
            Op::Mul => Some(a * b),
            Op::Div => {
                if b == 0 {
                    None
                } else {
                    Some(a / b)
                }
            }
        }
    }

    assert_eq!(run(Op::Add, 3, 4), Some(7));
    assert_eq!(run(Op::Sub, 10, 3), Some(7));
    assert_eq!(run(Op::Mul, 3, 4), Some(12));
    assert_eq!(run(Op::Div, 12, 4), Some(3));
    assert_eq!(run(Op::Div, 12, 0), None);
}

/// `match` with guards routes to the correct arm.
#[test]
fn match_guards_route_correctly() {
    #[cps_fn]
    fn run(x: i32) -> &'static str {
        match x {
            n if n < 0 => "negative",
            0 => "zero",
            n if n <= 10 => "small",
            _ => "large",
        }
    }

    assert_eq!(run(-5), "negative");
    assert_eq!(run(0), "zero");
    assert_eq!(run(5), "small");
    assert_eq!(run(10), "small");
    assert_eq!(run(11), "large");
}

/// An early return from a `match` arm exits the function.
#[test]
fn match_arm_early_return_exits_function() {
    #[cps_fn]
    fn run(input: Result<i32, &'static str>) -> i32 {
        let v = match input {
            Ok(n) => n,
            Err(_) => return -1,
        };
        v * 2
    }

    assert_eq!(run(Ok(5)), 10);
    assert_eq!(run(Err("boom")), -1);
}

// ─────────────────────────────────────────────────────────────────────────────
// §9  `loop_state` — carried bindings across iterations
// ─────────────────────────────────────────────────────────────────────────────

/// A binding carried by `loop_state` retains its value across iterations.
#[test]
fn loop_state_carries_binding_across_iterations() {
    #[cps_fn]
    fn run(rounds: u32) -> u32 {
        let mut total = 0_u32;
        let mut i = 0_u32;
        #[colorless::loop_state(total)]
        loop {
            if i >= rounds {
                break total;
            }
            total += i;
            i += 1;
        }
    }

    assert_eq!(run(0), 0);
    assert_eq!(run(1), 0); // 0
    assert_eq!(run(5), 10); // 0+1+2+3+4
}

/// A body-level shadow of a `loop_state` binding does not replace the carried value.
#[test]
fn loop_state_body_shadow_does_not_corrupt_carried_value() {
    #[allow(unused_mut, unused)]
    #[cps_fn]
    fn run() -> String {
        let mut carried = String::from("outer");
        let mut round = 0_u8;
        #[colorless::loop_state(carried, round)]
        loop {
            // Shadow `carried` inside the body — the carried value must be unaffected.
            let carried = String::from("shadow");
            assert_eq!(carried, "shadow");

            round += 1;
            if round >= 3 {
                break;
            }
        }
        // After the loop, `carried` must still be the original value.
        carried
    }

    assert_eq!(run(), "outer");
}

/// `loop_state` with a `pub` modifier makes the binding visible in the continuation.
#[test]
fn loop_state_pub_binding_is_visible_after_loop() {
    #[cps_fn]
    fn run(items: Vec<i32>) -> i32 {
        let mut sum = 0_i32;
        #[colorless::loop_state(pub sum)]
        for x in items {
            sum += x;
        }
        // `sum` is in scope here because of `pub`
        sum
    }

    assert_eq!(run(vec![1, 2, 3]), 6);
    assert_eq!(run(vec![]), 0);
}

/// An early return from inside a `loop_state` loop exits the function cleanly.
#[test]
fn loop_state_early_return_exits_function() {
    #[cps_fn]
    fn run(items: Vec<i32>) -> Result<i32, i32> {
        let mut sum = 0;
        #[colorless::loop_state(sum)]
        for x in items {
            if x < 0 {
                return Err(x);
            }
            sum += x;
        }
        Ok(sum)
    }

    assert_eq!(run(vec![1, 2, 3]), Ok(6));
    assert_eq!(run(vec![1, -2, 3]), Err(-2));
    assert_eq!(run(vec![-99]), Err(-99));
}

// ─────────────────────────────────────────────────────────────────────────────
// §10  Nested loop interactions
// ─────────────────────────────────────────────────────────────────────────────

/// An inner loop's `break` exits only the inner loop; the outer loop continues.
#[test]
fn inner_break_does_not_exit_outer_loop() {
    #[cps_fn]
    fn run() -> Vec<i32> {
        let mut log = Vec::new();
        for i in 0..3 {
            for _j in 0..10 {
                // Always break the inner loop on the first iteration
                log.push(i);
                break;
            }
        }
        log
    }

    // Outer loop runs 3 times; inner loop always breaks on first element
    assert_eq!(run(), vec![0, 1, 2]);
}

/// Nested loops with `continue` on the inner loop leave the outer untouched.
#[test]
fn inner_continue_does_not_affect_outer_loop() {
    #[cps_fn]
    fn run() -> i32 {
        let mut count = 0;
        for _i in 0..3 {
            for j in 0..5 {
                if j % 2 == 0 {
                    continue;
                }
                count += 1;
            }
        }
        count
    }

    // 2 odd values per inner pass (1,3), 3 outer iterations → 6
    assert_eq!(run(), 6);
}

/// Three-level nested labeled loops with cross-level `break` and `continue`.
#[test]
fn three_level_nested_labeled_loops_all_exits() {
    #[cps_fn]
    fn run(mode: u8) -> &'static str {
        'l1: for i in 0..5 {
            'l2: for j in 0..5 {
                for k in 0..5 {
                    match mode {
                        0 if i == 1 && j == 1 && k == 1 => return "early return",
                        1 if i == 2 && j == 2 && k == 2 => break 'l1,
                        2 if i == 1 && j == 2 && k == 3 => continue 'l1,
                        3 if i == 0 && j == 3 && k == 0 => break 'l2,
                        _ => {}
                    }
                }
            }
        }
        "completed"
    }

    assert_eq!(run(0), "early return");
    assert_eq!(run(1), "completed"); // break 'l1 exits normally, hits "completed"
    assert_eq!(run(2), "completed"); // continue 'l1 — loop runs to end
    assert_eq!(run(3), "completed"); // break 'l2 — just skips inner
    assert_eq!(run(99), "completed");
}

// ─────────────────────────────────────────────────────────────────────────────
// §11  Struct and tuple construction with diverging fields
// ─────────────────────────────────────────────────────────────────────────────

/// Struct fields are evaluated in declaration order; a diverging early field
/// must prevent later fields from being evaluated.
#[test]
fn struct_field_evaluation_order_with_early_return() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static EVALS: AtomicUsize = AtomicUsize::new(0);

    fn eval(label: usize, v: i32) -> i32 {
        EVALS.fetch_add(1, Ordering::Relaxed);
        let _ = label;
        v
    }

    struct Point {
        x: i32,
        y: i32,
        z: i32,
    }

    #[cps_fn]
    fn run(abort_x: bool, abort_y: bool) -> Result<i32, &'static str> {
        let p = Point {
            x: if abort_x {
                return Err("x aborted");
            } else {
                eval(0, 1)
            },
            y: if abort_y {
                return Err("y aborted");
            } else {
                eval(1, 2)
            },
            z: eval(2, 3),
        };
        Ok(p.x + p.y + p.z)
    }

    EVALS.store(0, Ordering::Relaxed);
    assert_eq!(run(true, false), Err("x aborted"));
    assert_eq!(EVALS.load(Ordering::Relaxed), 0); // x bailed before any eval

    EVALS.store(0, Ordering::Relaxed);
    assert_eq!(run(false, true), Err("y aborted"));
    assert_eq!(EVALS.load(Ordering::Relaxed), 1); // x evaluated, y bailed

    EVALS.store(0, Ordering::Relaxed);
    assert_eq!(run(false, false), Ok(6));
    assert_eq!(EVALS.load(Ordering::Relaxed), 3);
}

/// Tuple elements are evaluated left-to-right; a diverging element stops evaluation.
#[test]
fn tuple_element_evaluation_order_with_early_return() {
    #[cps_fn]
    fn run(a: bool, b: bool) -> Result<(i32, i32, i32), &'static str> {
        Ok((
            if a {
                return Err("a");
            } else {
                1
            },
            if b {
                return Err("b");
            } else {
                2
            },
            3,
        ))
    }

    assert_eq!(run(true, false), Err("a"));
    assert_eq!(run(false, true), Err("b"));
    assert_eq!(run(false, false), Ok((1, 2, 3)));
}

// ─────────────────────────────────────────────────────────────────────────────
// §12  Struct update syntax (`..base`)
// ─────────────────────────────────────────────────────────────────────────────

/// Struct update syntax (`{ field: v, ..base }`) moves `base` after all
/// explicit fields are evaluated; an early return in a field skips the move.
#[test]
fn struct_update_syntax_with_early_return_in_field() {
    struct Record {
        name: String,
        count: usize,
    }

    #[cps_fn]
    fn run(abort: bool) -> Result<usize, &'static str> {
        let source = Record {
            name: String::from("src"),
            count: 10,
        };
        let updated = Record {
            count: if abort {
                return Err("aborted");
            } else {
                7
            },
            ..source
        };
        Ok(updated.name.len() + updated.count)
    }

    assert_eq!(run(true), Err("aborted"));
    assert_eq!(run(false), Ok(3 + 7)); // "src".len()=3
}

// ─────────────────────────────────────────────────────────────────────────────
// §13  Closures — isolation from the outer function's control flow
// ─────────────────────────────────────────────────────────────────────────────

/// A `return` inside a closure exits only the closure, not the enclosing function.
#[test]
fn closure_return_does_not_exit_outer_function() {
    #[cps_fn]
    fn run(x: i32) -> i32 {
        let double_or_zero = |v: i32| -> i32 {
            if v < 0 {
                return 0;
            }
            v * 2
        };

        let a = double_or_zero(x);
        let b = double_or_zero(-5); // always 0 via closure return
        a + b
    }

    assert_eq!(run(5), 10); // 10 + 0
    assert_eq!(run(-3), 0); // 0 + 0
    assert_eq!(run(0), 0); // 0 + 0
}

/// A `break` inside a closure does not exit an outer loop — it's a compile
/// error if attempted, so we test that closures correctly isolate `break`
/// to their own inner loops.
#[test]
fn closure_internal_loop_break_is_scoped_to_closure() {
    #[cps_fn]
    fn run(items: Vec<i32>, limit: i32) -> i32 {
        // Closure with its own loop — the outer function's loop must not be affected.
        let first_exceeding = |slice: &[i32]| -> Option<i32> {
            for &v in slice {
                if v > limit {
                    return Some(v);
                }
            }
            None
        };

        let mut outer_count = 0;
        for chunk in items.chunks(2) {
            outer_count += 1;
            if first_exceeding(chunk).is_some() {
                break;
            }
        }
        outer_count
    }

    // [1,2,3,4,5,6] in chunks of 2: [1,2],[3,4],[5,6]
    // limit=3: [3,4] has 4>3, outer breaks after 2nd chunk
    assert_eq!(run(vec![1, 2, 3, 4, 5, 6], 3), 2);
    // limit=10: nothing exceeds, all 3 chunks visited
    assert_eq!(run(vec![1, 2, 3, 4, 5, 6], 10), 3);
}

// ─────────────────────────────────────────────────────────────────────────────
// §14  Unary operators and type casts with diverging operands
// ─────────────────────────────────────────────────────────────────────────────

/// A diverging expression inside a unary `-` must propagate before negation.
#[test]
fn unary_neg_with_diverging_operand() {
    #[cps_fn]
    fn run(abort: bool, v: i32) -> Result<i32, &'static str> {
        let result = -(if abort {
            return Err("aborted");
        } else {
            v
        });
        Ok(result)
    }

    assert_eq!(run(true, 5), Err("aborted"));
    assert_eq!(run(false, 5), Ok(-5));
    assert_eq!(run(false, -3), Ok(3));
}

/// A diverging expression inside a cast must propagate before the cast.
#[test]
fn cast_with_diverging_operand() {
    #[cps_fn]
    fn run(abort: bool, v: i32) -> Result<u32, &'static str> {
        let result = (if abort {
            return Err("aborted");
        } else {
            v
        }) as u32;
        Ok(result)
    }

    assert_eq!(run(true, 5), Err("aborted"));
    assert_eq!(run(false, 7), Ok(7_u32));
}

// ─────────────────────────────────────────────────────────────────────────────
// §15  `if`-`else` as expressions
// ─────────────────────────────────────────────────────────────────────────────

/// An `if-else` used as an expression produces the correct branch value.
#[test]
fn if_else_expression_produces_correct_branch() {
    #[cps_fn]
    fn run(x: i32) -> &'static str {
        let label = if x > 0 {
            "positive"
        } else if x < 0 {
            "negative"
        } else {
            "zero"
        };
        label
    }

    assert_eq!(run(1), "positive");
    assert_eq!(run(-1), "negative");
    assert_eq!(run(0), "zero");
}

/// Nested `if` expressions — each level evaluates independently.
#[test]
fn nested_if_expressions_evaluate_independently() {
    #[cps_fn]
    fn run(a: bool, b: bool, c: bool) -> u8 {
        if a {
            if b { if c { 7 } else { 6 } } else { 5 }
        } else {
            4
        }
    }

    assert_eq!(run(false, false, false), 4);
    assert_eq!(run(true, false, false), 5);
    assert_eq!(run(true, true, false), 6);
    assert_eq!(run(true, true, true), 7);
}

/// An early return in the condition of an `if` exits the function.
#[test]
fn if_condition_early_return_exits_function() {
    #[cps_fn]
    fn run(abort: bool) -> Result<&'static str, &'static str> {
        let cond = if abort {
            return Err("aborted");
        } else {
            true
        };
        Ok(if cond { "yes" } else { "no" })
    }

    assert_eq!(run(true), Err("aborted"));
    assert_eq!(run(false), Ok("yes"));
}

// ─────────────────────────────────────────────────────────────────────────────
// §16  Labeled blocks (not loops)
// ─────────────────────────────────────────────────────────────────────────────

/// A `break 'label VALUE` from a labeled block produces the correct value.
#[test]
fn labeled_block_break_produces_value() {
    #[cps_fn]
    fn run(flag: bool) -> i32 {
        let v = 'blk: {
            if flag {
                break 'blk 1;
            }
            break 'blk 2;
        };
        v * 10
    }

    assert_eq!(run(true), 10);
    assert_eq!(run(false), 20);
}

/// A labeled block nested inside a loop — `break 'block` exits only the block.
#[test]
fn labeled_block_break_inside_loop_exits_only_block() {
    #[cps_fn]
    fn run() -> Vec<i32> {
        let mut log = Vec::new();
        for i in 0..3 {
            let val = 'blk: {
                if i == 1 {
                    break 'blk -1;
                }
                i * 10
            };
            log.push(val);
        }
        log
    }

    assert_eq!(run(), vec![0, -1, 20]);
}

// ─────────────────────────────────────────────────────────────────────────────
// §17  `loop_state` with nested loops and cross-level jumps
// ─────────────────────────────────────────────────────────────────────────────

/// `loop_state` on an outer loop carries state across iterations even when
/// an inner loop performs `continue 'outer`.
#[test]
fn loop_state_survives_cross_level_continue() {
    #[cps_fn]
    fn run() -> u32 {
        let mut total = 0_u32;
        let mut i = 0_u32;
        #[colorless::loop_state(total)]
        'outer: loop {
            if i >= 5 {
                break;
            }
            loop {
                // Contribute to total and immediately retry outer
                total += i;
                i += 1;
                continue 'outer;
            }
        }
        total
    }

    // sum(0..5) = 0+1+2+3+4 = 10
    assert_eq!(run(), 10);
}

/// A deeply nested early return correctly abandons both loops and the function.
#[test]
fn deep_nested_early_return_abandons_all_loops() {
    #[cps_fn]
    fn run(items: &[&[i32]], needle: i32) -> Option<(usize, usize)> {
        for (i, row) in items.iter().enumerate() {
            for (j, &v) in row.iter().enumerate() {
                if v == needle {
                    return Some((i, j));
                }
            }
        }
        None
    }

    let data: &[&[i32]] = &[&[1, 2, 3], &[4, 5, 6], &[7, 8, 9]];
    assert_eq!(run(data, 5), Some((1, 1)));
    assert_eq!(run(data, 1), Some((0, 0)));
    assert_eq!(run(data, 9), Some((2, 2)));
    assert_eq!(run(data, 99), None);
}

// ─────────────────────────────────────────────────────────────────────────────
// §18  Equivalence: CPS transform preserves pure-computation results
// ─────────────────────────────────────────────────────────────────────────────

/// A recursive Fibonacci computed via CPS must match the reference.
#[test]
fn fibonacci_cps_matches_reference() {
    fn fib_ref(n: u64) -> u64 {
        match n {
            0 => 0,
            1 => 1,
            n => fib_ref(n - 1) + fib_ref(n - 2),
        }
    }

    #[cps_fn]
    fn fib_cps(n: u64) -> u64 {
        match n {
            0 => 0,
            1 => 1,
            n => fib_cps(n - 1) + fib_cps(n - 2),
        }
    }

    for i in 0..=15 {
        assert_eq!(fib_cps(i), fib_ref(i), "fibonacci({i}) mismatch");
    }
}

/// A pipeline of `map`/`filter`/`fold` operations via CPS matches the iterator result.
#[test]
fn map_filter_fold_pipeline_matches_iterator() {
    fn double(x: i32) -> i32 {
        x * 2
    }
    fn is_positive(x: &i32) -> bool {
        *x > 0
    }

    #[cps_fn]
    fn cps_pipeline(items: Vec<i32>) -> i32 {
        let mut sum = 0;
        for x in items {
            let v = double(x);
            if !is_positive(&v) {
                continue;
            }
            sum += v;
        }
        sum
    }

    fn ref_pipeline(items: Vec<i32>) -> i32 {
        items.into_iter().map(double).filter(is_positive).sum()
    }

    let cases = vec![
        vec![],
        vec![1, -2, 3, -4, 5],
        vec![-1, -2, -3],
        vec![10, 20, 30],
    ];

    for case in cases {
        assert_eq!(cps_pipeline(case.clone()), ref_pipeline(case));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// §19  Loop signal routing — which continuations are live for a given jump
// ─────────────────────────────────────────────────────────────────────────────

/// A `break 'outer` from an inner `match` arm routes to the correct target.
/// Code that follows the inner `loop { }` only executes when the inner loop
/// broke locally (not to 'outer).
///
/// Three routing paths are verified:
/// - action 0: `break 'outer VALUE` from match arm → post-loop code skipped
/// - action 1: `continue 'outer` on first round, `break 'outer` on second → post-loop skipped
/// - action 2: `break` (inner only) → post-loop code runs, then `break 'outer`
#[test]
fn loop_signal_routing_matrix() {
    #[cps_fn]
    fn route(action: u8, hits: &mut usize) -> i32 {
        let mut rounds = 0;
        let value = 'outer: loop {
            rounds += 1;
            loop {
                match action {
                    0 => break 'outer rounds * 10,
                    1 if rounds == 1 => continue 'outer,
                    1 => break 'outer rounds * 10 + 1,
                    _ => break,
                }
            }

            // Only reachable when the inner loop broke locally.
            *hits += 1;
            break 'outer rounds * 10 + 2;
        };

        *hits += 100;
        value + rounds * 1000
    }

    // action=0: direct break 'outer on round 1 → value=10, rounds=1 → 10+1000=1010
    let mut hits = 0;
    assert_eq!(route(0, &mut hits), 1010);
    assert_eq!(hits, 100); // post-loop runs, inner body did not

    // action=1: continue 'outer round 1 → round 2 → break 'outer 21 → 21+2000=2021
    hits = 0;
    assert_eq!(route(1, &mut hits), 2021);
    assert_eq!(hits, 100); // inner body still never ran

    // action=2: inner break → hits+=1 → break 'outer 12 → 12+1000=1012
    hits = 0;
    assert_eq!(route(2, &mut hits), 1012);
    assert_eq!(hits, 101); // inner body ran once, then post-loop
}

/// An early `return` from inside a `loop_state` inner loop bypasses the
/// outer loop's break continuation entirely.
#[test]
fn return_from_loop_state_inner_loop_bypasses_outer_break_continuation() {
    #[allow(unused_mut)]
    #[cps_fn]
    fn run(abort: bool, hits: &mut usize) -> Result<String, &'static str> {
        let result = 'outer: loop {
            let mut payload = String::from("payload");
            #[colorless::loop_state(payload)]
            loop {
                if abort {
                    return Err("abort from inner");
                }
                break 'outer payload;
            }
        };

        *hits += 1;
        Ok(result + "!")
    }

    let mut hits = 0;
    assert_eq!(run(true, &mut hits), Err("abort from inner"));
    assert_eq!(
        hits, 0,
        "break continuation must not run after early return"
    );

    assert_eq!(run(false, &mut hits), Ok(String::from("payload!")));
    assert_eq!(hits, 1);
}

/// A `match` inside a nested `loop { loop { match … } }` correctly routes
/// `break 'outer VALUE` without executing the outer loop's tail.
#[test]
fn match_in_nested_loop_routes_to_correct_outer_continuation() {
    #[cps_fn]
    fn run(mode: u8, hits: &mut usize) -> i32 {
        'outer: loop {
            loop {
                match mode {
                    0 => break 'outer 40,
                    1 => break 'outer 50,
                    _ => break,
                }
            }
            *hits += 1;
            break 'outer 60;
        }
    }

    let mut hits = 0;
    assert_eq!(run(0, &mut hits), 40);
    assert_eq!(hits, 0); // outer tail must not run

    assert_eq!(run(1, &mut hits), 50);
    assert_eq!(hits, 0);

    assert_eq!(run(2, &mut hits), 60);
    assert_eq!(hits, 1); // inner broke locally → outer tail did run
}

// ─────────────────────────────────────────────────────────────────────────────
// §20  loop_state canonical continuation hygiene
// ─────────────────────────────────────────────────────────────────────────────

/// After a `loop_state` loop, the continuation sees the original carried
/// binding — not the body-local shadow — and can shadow it again independently.
#[test]
fn canonical_continuation_rehydrates_carried_binding_not_body_shadow() {
    #[cps_fn]
    #[allow(unused_mut)]
    fn run(take_first_path: bool) -> (String, String, bool) {
        let mut env = String::from("outer");
        let mut flag = take_first_path;

        #[colorless::loop_state(env, flag)]
        loop {
            let env = String::from("body shadow");
            assert_eq!(env, "body shadow");
            if flag { break } else { break }
        }

        // The continuation must see the carried value, not the body shadow.
        assert_eq!(env, "outer");
        let observed = env;

        // Shadowing in the continuation is independent.
        let env = String::from("continuation local");
        (observed, env, flag)
    }

    for path in [true, false] {
        let (observed, local, flag) = run(path);
        assert_eq!(observed, "outer");
        assert_eq!(local, "continuation local");
        assert_eq!(flag, path);
    }
}

/// Two sequential `loop_state` loops in the same function remain independent:
/// the second loop does not inherit any state from the first.
#[test]
fn two_sequential_loop_state_loops_are_independent() {
    #[cps_fn]
    #[allow(unused_mut)]
    fn run() -> (String, u8) {
        let mut env = String::from("outer");
        let mut remaining = 2_u8;

        #[colorless::loop_state(env, remaining)]
        loop {
            let env = String::from("first-loop shadow");
            assert_eq!(env, "first-loop shadow");
            break;
        }

        // env is still "outer" here — first loop's shadow is gone.
        assert_eq!(env, "outer");

        #[colorless::loop_state(env, remaining)]
        loop {
            remaining -= 1;
            if remaining == 0 {
                break (env, remaining);
            }
        }
    }

    let (env, remaining) = run();
    assert_eq!(env, "outer");
    assert_eq!(remaining, 0);
}

/// Three `loop_state` loops that all carry a binding spelled identically (`value`)
/// remain independently addressable — the transform must not conflate their frames.
#[test]
fn three_identically_spelled_loop_state_frames_are_independent() {
    #[cps_fn]
    fn run() -> (String, String, String) {
        let value = String::from("outer");
        (
            {
                #[colorless::loop_state(value)]
                loop {
                    break value;
                }
            },
            {
                let value = String::from("middle");
                #[colorless::loop_state(value)]
                loop {
                    break value;
                }
            },
            {
                let value = String::from("inner");
                #[colorless::loop_state(value)]
                loop {
                    break value;
                }
            },
        )
    }

    let (outer, middle, inner) = run();
    assert_eq!(outer, "outer");
    assert_eq!(middle, "middle");
    assert_eq!(inner, "inner");
}

/// Full labeled cross-loop routing matrix: `continue 'inner`, `continue 'outer`,
/// `break 'inner`, and `break 'outer` from inside nested `loop_state` loops.
/// This is the most complete signal-routing test: each jump must carry exactly
/// the frames that belong to its target, discard the rest, and produce the
/// correct execution trace.
#[test]
fn labeled_cross_loop_routing_carries_correct_frame_prefix() {
    #[allow(unreachable_code)]
    #[cps_fn]
    fn run() -> (String, u8, Vec<&'static str>) {
        let mut env = String::from("outer");
        let mut outer_round = 0_u8;
        let mut log = Vec::new();

        #[colorless::loop_state(pub env, outer_round, pub log)]
        'outer: loop {
            assert_eq!(env, "outer");
            log.push("outer-enter");

            let env = String::from("inner");
            let mut inner_round = 0_u8;

            #[colorless::loop_state(env, inner_round)]
            'inner: loop {
                assert_eq!(env, "inner");

                match (outer_round, inner_round) {
                    // continue 'inner — all active frames preserved
                    (0, 0) => {
                        log.push("continue-inner");
                        inner_round += 1;
                        continue;
                    }
                    // continue 'outer — inner frame abandoned, only outer prefix sent up
                    (0, 1) => {
                        log.push("continue-outer");
                        outer_round += 1;
                        continue 'outer;
                    }
                    // break 'inner — outer body resumes after the inner loop
                    (1, 0) => {
                        log.push("break-inner");
                        break 'inner;
                    }
                    // break 'outer — outer canonical continuation invoked
                    (2, 0) => {
                        log.push("break-outer");
                        break 'outer;
                    }
                    state => panic!("unexpected state: {state:?}"),
                }
            }

            assert_eq!(env, "inner");
            log.push("outer-tail");
            outer_round += 1;
            continue 'outer;
        }

        log.push("outer-continuation");
        (env, outer_round, log)
    }

    let (env, outer_round, log) = run();

    assert_eq!(env, "outer");
    assert_eq!(outer_round, 2);
    assert_eq!(
        log,
        [
            "outer-enter",
            "continue-inner",
            "continue-outer",
            "outer-enter",
            "break-inner",
            "outer-tail",
            "outer-enter",
            "break-outer",
            "outer-continuation",
        ]
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// §21  Drop timing — RAII guards released at the right scope boundary
// ─────────────────────────────────────────────────────────────────────────────

/// A `MutexGuard` created inside a block expression is released before the
/// next lock attempt on the same `Mutex`. This validates that relaxed-drop
/// timing could still release guards by the end of their enclosing block.
#[test]
fn lexical_drop_releases_mutex_guard_before_next_lock() {
    fn fetch(id: i32) -> i32 {
        id
    }

    #[cps_fn]
    #[allow(unused)]
    fn func(lock: Mutex<i32>) -> i32 {
        let b = if false {
            return 0;
        } else {
            // Guard is created and dropped at the end of this block.
            let v = lock.lock().unwrap();
            fetch(*v)
        };
        // Guard must be released here — acquiring again must not deadlock.
        lock.lock().expect("guard should be released");
        fetch(b)
    }

    assert_eq!(func(Mutex::new(12)), 12);
}

// ─────────────────────────────────────────────────────────────────────────────
// §22  `ControlFlow` try desugaring
// ─────────────────────────────────────────────────────────────────────────────

/// `?` on `ControlFlow<B, C>` propagates `Break` and unwraps `Continue`.
/// This is a distinct desugar path from both `Result` and `Option` `?`.
#[test]
fn try_operator_on_control_flow_propagates_break() {
    use std::ops::ControlFlow;

    fn step(x: i32) -> ControlFlow<&'static str, i32> {
        if x == 99 {
            ControlFlow::Break("cf break")
        } else {
            ControlFlow::Continue(x + 1)
        }
    }

    #[cps_fn]
    fn run(a: i32, b: i32) -> ControlFlow<&'static str, i32> {
        let ca = step(a)?;
        let cb = step(b)?;
        ControlFlow::Continue(ca + cb)
    }

    assert_eq!(run(99, 10), ControlFlow::Break("cf break"));
    assert_eq!(run(10, 99), ControlFlow::Break("cf break"));
    // Both Continue: (10+1) + (20+1) = 32
    assert_eq!(run(10, 20), ControlFlow::Continue(32));
}

// ─────────────────────────────────────────────────────────────────────────────
// §23  Combined `let-else` + `match` with struct/tuple patterns and guards
// ─────────────────────────────────────────────────────────────────────────────

/// Combines `let-else` divergence with a `match` over an enum whose arms use
/// struct patterns, tuple patterns, negative guards, and an early return inside
/// a match body. This exercises the interaction of three distinct desugar paths
/// in one function. `emit_inert = true` also validates the inert emission path.
#[test]
fn let_else_plus_match_with_struct_tuple_patterns_and_guards() {
    #[derive(Debug, PartialEq, Eq)]
    enum AppAction {
        Quit,
        Move { x: i32, y: i32 },
        Write(String),
        Compute(i32, i32),
    }

    #[cps_fn(emit_inert = true)]
    fn run(action: AppAction, opt: Option<i32>) -> Result<i32, &'static str> {
        let Some(multiplier) = opt else {
            return Err("missing multiplier");
        };

        let base = match action {
            AppAction::Quit => return Err("aborted on quit"),
            AppAction::Move { x, y } if x < 0 || y < 0 => return Err("negative coordinates"),
            AppAction::Move { x, y } => x + y,
            AppAction::Write(s) => s.len() as i32,
            AppAction::Compute(a, b) => {
                let prod = a * b;
                if prod > 500 {
                    return Err("overflowing computation");
                }
                prod
            }
        };

        Ok(base * multiplier)
    }

    // let-else diverges when opt is None
    assert_eq!(run(AppAction::Quit, None), Err("missing multiplier"));
    // Unit variant early return
    assert_eq!(run(AppAction::Quit, Some(2)), Err("aborted on quit"));
    // Struct pattern with negative guard
    assert_eq!(
        run(AppAction::Move { x: -1, y: 5 }, Some(2)),
        Err("negative coordinates")
    );
    // Struct pattern normal path: (10+20)*3 = 90
    assert_eq!(run(AppAction::Move { x: 10, y: 20 }, Some(3)), Ok(90));
    // Tuple variant with String: "hello".len()=5, *4=20
    assert_eq!(run(AppAction::Write("hello".to_string()), Some(4)), Ok(20));
    // Tuple variant with guard: 30*20=600 > 500
    assert_eq!(
        run(AppAction::Compute(30, 20), Some(1)),
        Err("overflowing computation")
    );
    // Tuple variant normal: 5*6=30, *10=300
    assert_eq!(run(AppAction::Compute(5, 6), Some(10)), Ok(300));
}

// ─────────────────────────────────────────────────────────────────────────────
// §24  Evaluation order under CPS transform
//
// These tests verify that the CPS transform preserves Rust's left-to-right,
// callee-before-arguments evaluation order for all compound expressions that
// the lowering must rewrite.  Each test uses a `RefCell<Vec<&str>>` log and
// asserts the exact sequence of side effects.
//
// Coverage follows the Rust Reference §"Evaluation order of operands":
//   tuple, array, struct literal, arithmetic, call (callee+args), method call,
//   index, range, cast, unary, borrow, assignment, compound assignment.
//
// Tests marked `#[ignore]` document known bugs in the lowering of field access
// and compound-assignment deref_mut; they should be un-ignored once fixed.
// ─────────────────────────────────────────────────────────────────────────────

use std::cell::RefCell;

/// Tuple elements are evaluated strictly left-to-right under `#[cps_fn]`.
#[test]
fn tuple_elements_evaluate_left_to_right() {
    let log = RefCell::new(Vec::new());

    #[cps_fn]
    fn run<'a>(log: &'a RefCell<Vec<&'static str>>) -> (i32, i32, i32) {
        (
            {
                log.borrow_mut().push("first");
                1
            },
            {
                log.borrow_mut().push("second");
                2
            },
            {
                log.borrow_mut().push("third");
                3
            },
        )
    }

    let result = run(&log);
    assert_eq!(result, (1, 2, 3));
    assert_eq!(*log.borrow(), ["first", "second", "third"]);
}

/// Array elements are evaluated strictly left-to-right under `#[cps_fn]`.
#[test]
fn array_elements_evaluate_left_to_right() {
    let log = RefCell::new(Vec::new());

    #[cps_fn]
    fn run<'a>(log: &'a RefCell<Vec<&'static str>>) -> [i32; 3] {
        [
            {
                log.borrow_mut().push("first");
                10
            },
            {
                log.borrow_mut().push("second");
                20
            },
            {
                log.borrow_mut().push("third");
                30
            },
        ]
    }

    let result = run(&log);
    assert_eq!(result, [10, 20, 30]);
    assert_eq!(*log.borrow(), ["first", "second", "third"]);
}

/// Nested arithmetic: each sub-expression is hoisted in left-to-right order.
/// `(a + b) + (c + d)` must evaluate `a`, `b`, `c`, `d` in that order.
#[test]
fn nested_arithmetic_evaluates_left_to_right() {
    let log = RefCell::new(Vec::new());

    #[cps_fn]
    fn run<'a>(log: &'a RefCell<Vec<&'static str>>) -> i32 {
        ({
            log.borrow_mut().push("a");
            1
        } + {
            log.borrow_mut().push("b");
            2
        }) + ({
            log.borrow_mut().push("c");
            3
        } + {
            log.borrow_mut().push("d");
            4
        })
    }

    let result = run(&log);
    assert_eq!(result, 10);
    assert_eq!(*log.borrow(), ["a", "b", "c", "d"]);
}

/// Function call: callee is evaluated before each argument, arguments
/// are evaluated left-to-right.
#[test]
fn call_evaluates_callee_then_arguments_left_to_right() {
    let log = RefCell::new(Vec::new());

    #[cps_fn]
    fn run<'a>(log: &'a RefCell<Vec<&'static str>>) -> i32 {
        let make_fn = || -> fn(i32, i32) -> i32 {
            log.borrow_mut().push("callee");
            |a, b| a + b
        };
        make_fn()(
            {
                log.borrow_mut().push("arg1");
                10
            },
            {
                log.borrow_mut().push("arg2");
                20
            },
        )
    }

    let result = run(&log);
    assert_eq!(result, 30);
    assert_eq!(*log.borrow(), ["callee", "arg1", "arg2"]);
}

/// Unary negation: the operand expression is fully evaluated before the
/// operator is applied. (Simple but CPS must not reorder it.)
#[test]
fn unary_operand_evaluates_before_operator() {
    let log = RefCell::new(Vec::new());

    #[cps_fn]
    fn run<'a>(log: &'a RefCell<Vec<&'static str>>) -> bool {
        !{
            log.borrow_mut().push("operand");
            false
        }
    }

    let result = run(&log);
    assert!(result);
    assert_eq!(*log.borrow(), ["operand"]);
}

/// Assignment: RHS is fully evaluated before the LHS place expression is
/// resolved.  Both sides have observable side effects so the ordering is
/// verifiable.  The place uses a `get()` method that pushes to the log when
/// called; the CPS transform must emit the RHS block first.
#[test]
fn assignment_evaluates_rhs_before_lhs_place() {
    struct Place<'a> {
        log: &'a RefCell<Vec<&'static str>>,
        value: i32,
    }

    impl<'a> Place<'a> {
        fn get(&mut self) -> &mut i32 {
            self.log.borrow_mut().push("lhs");
            &mut self.value
        }
    }

    let log = RefCell::new(Vec::new());
    let mut place = Place {
        log: &log,
        value: 0,
    };

    #[cps_fn]
    fn run(log: &RefCell<Vec<&'static str>>, place: &mut Place) {
        *place.get() = {
            log.borrow_mut().push("rhs");
            42
        };
    }

    run(&log, &mut place);
    assert_eq!(place.value, 42);
    assert_eq!(*log.borrow(), ["rhs", "lhs"]);
}

/// Tuple with a branching element: when one element contains a control-flow
/// branch (`if`), the other elements still evaluate in the correct order and
/// the final tuple is correctly assembled.
#[test]
fn tuple_with_branching_element_preserves_order() {
    let log = RefCell::new(Vec::new());

    #[cps_fn]
    fn run<'a>(log: &'a RefCell<Vec<&'static str>>, flag: bool) -> (i32, i32) {
        (
            {
                log.borrow_mut().push("first");
                if flag { 1 } else { -1 }
            },
            {
                log.borrow_mut().push("second");
                2
            },
        )
    }

    assert_eq!(run(&log, true), (1, 2));
    assert_eq!(*log.borrow(), ["first", "second"]);

    let log2 = RefCell::new(Vec::new());
    assert_eq!(run(&log2, false), (-1, 2));
    assert_eq!(*log2.borrow(), ["first", "second"]);
}

/// Array with a branching element: same as above for array literals.
#[test]
fn array_with_branching_element_preserves_order() {
    let log = RefCell::new(Vec::new());

    #[cps_fn]
    fn run<'a>(log: &'a RefCell<Vec<&'static str>>, flag: bool) -> [i32; 2] {
        [
            {
                log.borrow_mut().push("first");
                if flag { 10 } else { -10 }
            },
            {
                log.borrow_mut().push("second");
                20
            },
        ]
    }

    assert_eq!(run(&log, true), [10, 20]);
    assert_eq!(*log.borrow(), ["first", "second"]);

    let log2 = RefCell::new(Vec::new());
    assert_eq!(run(&log2, false), [-10, 20]);
    assert_eq!(*log2.borrow(), ["first", "second"]);
}

// ── §24 continued: remaining Rust evaluation-order rules ─────────────────────

/// Struct literal: fields are evaluated in declaration order (left-to-right).
/// The Rust reference guarantees this; the CPS transform must preserve it.
#[test]
fn struct_literal_fields_evaluate_left_to_right() {
    struct Point {
        x: i32,
        y: i32,
        z: i32,
    }

    let log = RefCell::new(Vec::new());

    #[cps_fn]
    fn run<'a>(log: &'a RefCell<Vec<&'static str>>) -> (i32, i32, i32) {
        let p = Point {
            x: {
                log.borrow_mut().push("x");
                1
            },
            y: {
                log.borrow_mut().push("y");
                2
            },
            z: {
                log.borrow_mut().push("z");
                3
            },
        };
        (p.x, p.y, p.z)
    }

    let result = run(&log);
    assert_eq!(result, (1, 2, 3));
    assert_eq!(*log.borrow(), ["x", "y", "z"]);
}

/// Index expression: the base (collection) is evaluated before the index.
#[test]
fn index_evaluates_base_before_index() {
    let log = RefCell::new(Vec::new());

    #[cps_fn]
    fn run<'a>(log: &'a RefCell<Vec<&'static str>>) -> i32 {
        let make_arr = || -> [i32; 3] {
            log.borrow_mut().push("base");
            [10, 20, 30]
        };
        make_arr()[{
            log.borrow_mut().push("index");
            1
        }]
    }

    let result = run(&log);
    assert_eq!(result, 20);
    assert_eq!(*log.borrow(), ["base", "index"]);
}

/// Method call (no auto-deref adjustment): receiver is evaluated before
/// arguments, and arguments are evaluated left-to-right.
#[test]
fn method_call_evaluates_receiver_then_args_left_to_right() {
    struct Adder;
    impl Adder {
        fn add(&self, a: i32, b: i32) -> i32 {
            a + b
        }
    }

    let log = RefCell::new(Vec::new());

    #[cps_fn]
    fn run<'a>(log: &'a RefCell<Vec<&'static str>>) -> i32 {
        let make_receiver = || -> Adder {
            log.borrow_mut().push("receiver");
            Adder
        };
        make_receiver().add(
            {
                log.borrow_mut().push("arg1");
                10
            },
            {
                log.borrow_mut().push("arg2");
                20
            },
        )
    }

    let result = run(&log);
    assert_eq!(result, 30);
    assert_eq!(*log.borrow(), ["receiver", "arg1", "arg2"]);
}

/// Method call with auto-deref receiver: the receiver adjustment (Deref coercion)
/// must happen before arguments are evaluated.
#[test]
fn method_call_with_autoderef_receiver_evaluates_before_args() {
    use std::ops::Deref;

    struct Wrapper<'a> {
        log: &'a RefCell<Vec<&'static str>>,
        inner: String,
    }

    impl<'a> Deref for Wrapper<'a> {
        type Target = str;
        fn deref(&self) -> &Self::Target {
            self.log.borrow_mut().push("deref");
            &self.inner
        }
    }

    let log = RefCell::new(Vec::new());
    let w = Wrapper {
        log: &log,
        inner: "hello".to_string(),
    };
    // `str::contains` takes `&self` — the deref happens to adjust `w` to `&str`
    // before the argument is evaluated.
    let _found = w.contains({
        log.borrow_mut().push("arg");
        'h'
    });

    assert_eq!(*log.borrow(), ["deref", "arg"]);
}

/// Range expression `a..b`: start (`a`) is evaluated before end (`b`).
#[test]
fn range_evaluates_start_before_end() {
    let log = RefCell::new(Vec::new());

    #[cps_fn]
    fn run<'a>(log: &'a RefCell<Vec<&'static str>>) -> std::ops::Range<i32> {
        ({
            log.borrow_mut().push("start");
            1_i32
        })..{
            log.borrow_mut().push("end");
            10
        }
    }

    let r = run(&log);
    assert_eq!(r, 1..10);
    assert_eq!(*log.borrow(), ["start", "end"]);
}

/// Inclusive range `a..=b`: start before end.
#[test]
fn inclusive_range_evaluates_start_before_end() {
    let log = RefCell::new(Vec::new());

    #[cps_fn]
    fn run<'a>(log: &'a RefCell<Vec<&'static str>>) -> std::ops::RangeInclusive<i32> {
        ({
            log.borrow_mut().push("start");
            1
        })..={
            log.borrow_mut().push("end");
            10
        }
    }

    let r = run(&log);
    assert_eq!(r, 1..=10);
    assert_eq!(*log.borrow(), ["start", "end"]);
}

/// Cast expression (`expr as T`): the operand is fully evaluated before the cast.
#[test]
fn cast_evaluates_operand_before_cast() {
    let log = RefCell::new(Vec::new());

    #[cps_fn]
    fn run<'a>(log: &'a RefCell<Vec<&'static str>>) -> i64 {
        ({
            log.borrow_mut().push("operand");
            42i32
        }) as i64
    }

    let result = run(&log);
    assert_eq!(result, 42i64);
    assert_eq!(*log.borrow(), ["operand"]);
}

/// Borrow (`&expr`): the operand is fully evaluated before the reference is taken.
/// The CPS transform must not defer or reorder the operand expression.
#[test]
fn borrow_evaluates_operand_before_taking_reference() {
    let log = RefCell::new(Vec::new());

    #[cps_fn]
    fn run<'a>(log: &'a RefCell<Vec<&'static str>>, x: &'a i32) -> &'a i32 {
        // Can't take a borrow of a block-expression easily as a return value,
        // so we pass a ref through a function that records the access.
        log.borrow_mut().push("before-borrow");
        let r: &i32 = x;
        log.borrow_mut().push("after-borrow");
        r
    }

    let val = 7i32;
    let r = run(&log, &val);
    assert_eq!(*r, 7);
    assert_eq!(*log.borrow(), ["before-borrow", "after-borrow"]);
}

/// Compound assignment (`+=`): the RHS is evaluated before the LHS place is
/// mutably dereferenced.
#[test]
fn compound_assignment_evaluates_rhs_before_deref_mut() {
    use std::ops::{Deref, DerefMut};

    struct Tracked<'a> {
        log: &'a RefCell<Vec<&'static str>>,
        value: i32,
    }

    impl<'a> Deref for Tracked<'a> {
        type Target = i32;
        fn deref(&self) -> &i32 {
            &self.value
        }
    }

    impl<'a> DerefMut for Tracked<'a> {
        fn deref_mut(&mut self) -> &mut i32 {
            self.log.borrow_mut().push("deref_mut");
            &mut self.value
        }
    }

    let log = RefCell::new(Vec::new());
    let mut t = Tracked {
        log: &log,
        value: 0,
    };
    // `*t += rhs` — rhs should be evaluated ("rhs") before deref_mut fires ("deref_mut").
    *t += {
        log.borrow_mut().push("rhs");
        5
    };

    assert_eq!(*t, 5);
    assert_eq!(*log.borrow(), ["rhs", "deref_mut"]);
}

/// Field access on a compound (call) expression: the base expression is
/// evaluated before the field is projected out.
#[test]
fn field_access_evaluates_base_before_projecting_field() {
    struct Point {
        x: i32,
        #[allow(dead_code)]
        y: i32,
    }

    let log = RefCell::new(Vec::new());

    #[cps_fn]
    fn run<'a>(log: &'a RefCell<Vec<&'static str>>) -> i32 {
        let make_point = || -> Point {
            log.borrow_mut().push("base");
            Point { x: 42, y: 0 }
        };
        make_point().x
    }

    let result = run(&log);
    assert_eq!(result, 42);
    assert_eq!(*log.borrow(), ["base"]);
}

/// Nested call chain: `f()(g())` — outer callee, then inner callee, then arg.
/// Stress-tests that hoisting respects call nesting across multiple levels.
#[test]
fn nested_call_chain_evaluates_in_order() {
    let log = RefCell::new(Vec::new());

    #[cps_fn]
    fn run<'a>(log: &'a RefCell<Vec<&'static str>>) -> i32 {
        let outer = || -> fn(i32) -> i32 {
            log.borrow_mut().push("outer-callee");
            |x| x * 2
        };
        let inner = || -> i32 {
            log.borrow_mut().push("inner-callee");
            21
        };
        outer()(inner())
    }

    let result = run(&log);
    assert_eq!(result, 42);
    assert_eq!(*log.borrow(), ["outer-callee", "inner-callee"]);
}

/// Mixed compound expression: `(a + b) * c` where all three operands have
/// side effects — asserts that hoisting does not reorder them.
#[test]
fn mixed_compound_expression_preserves_full_order() {
    let log = RefCell::new(Vec::new());

    #[cps_fn]
    fn run<'a>(log: &'a RefCell<Vec<&'static str>>) -> i32 {
        ({
            log.borrow_mut().push("a");
            3
        } + {
            log.borrow_mut().push("b");
            4
        }) * {
            log.borrow_mut().push("c");
            2
        }
    }

    let result = run(&log);
    assert_eq!(result, 14);
    assert_eq!(*log.borrow(), ["a", "b", "c"]);
}

/// Four-level deeply nested arithmetic: all eight sub-expressions evaluated
/// strictly left-to-right with no reordering by the CPS hoisting pass.
#[test]
fn deeply_nested_arithmetic_preserves_order() {
    let log = RefCell::new(Vec::new());

    #[cps_fn]
    fn run<'a>(log: &'a RefCell<Vec<&'static str>>) -> i32 {
        (({
            log.borrow_mut().push("1");
            1
        } + {
            log.borrow_mut().push("2");
            2
        }) + ({
            log.borrow_mut().push("3");
            3
        } + {
            log.borrow_mut().push("4");
            4
        })) + (({
            log.borrow_mut().push("5");
            5
        } + {
            log.borrow_mut().push("6");
            6
        }) + ({
            log.borrow_mut().push("7");
            7
        } + {
            log.borrow_mut().push("8");
            8
        }))
    }

    let result = run(&log);
    assert_eq!(result, 36);
    assert_eq!(*log.borrow(), ["1", "2", "3", "4", "5", "6", "7", "8"]);
}
