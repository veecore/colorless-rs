//! Brute-force compile-and-run checks for unusual syntax combinations.
//!
//! These tests exist for two purposes:
//!
//! 1. **Compile checks** — verify the CPS transform accepts exotic but legal
//!    Rust syntax (loop-as-expression inside `while let`, tuple `let-else`,
//!    `return LOOP { … }`, etc.). Even functions that are never called exercise
//!    the macro expander at compile time.
//!
//! 2. **No-panic runtime checks** — for the subset that terminates, a `#[test]`
//!    wrapper verifies the generated code actually runs to completion without
//!    panicking. No specific output value is asserted; correctness of output
//!    for these constructs is covered by `semantics.rs`.
//!
//! # Compile-only functions
//!
//! Functions marked *compile-only* contain unconditional infinite loops (e.g.
//! `if true { continue 'a; }`) and would hang if called. They are plain free
//! functions — the compiler still parses, type-checks, and lowers them through
//! the CPS transform, which is the check we care about.

use dev_macros::cps_fn;
use std::marker::PhantomData;

fn unify_value<T>(a: T, _b: PhantomData<T>) -> T {
    a
}

// ─────────────────────────────────────────────────────────────────────────────
// Compile-only: infinite loops that exercise cross-label routing
// ─────────────────────────────────────────────────────────────────────────────

/// Compile-only. Two nested `loop_state` loops where the inner loop always
/// does `continue 'outer`. Verifies that the transform correctly routes a
/// `continue 'a` from inside a `loop_state 'b` body.
#[cps_fn]
#[allow(dead_code, unreachable_code, unused_mut)]
fn nested_loop_state_always_continue_outer() {
    let mut env = String::from("outer");
    #[colorless::loop_state(env)]
    'a: loop {
        assert_eq!(env, "outer");
        let mut ee = String::from("outer_ee");
        #[colorless::loop_state(ee)]
        'b: loop {
            if true {
                continue 'a;
            }

            // Unreachable — still must compile correctly.
            let mut inner_env = String::from("inner");
            loop {
                assert_eq!(inner_env, "inner");
                if true {
                    continue 'a;
                } else {
                    continue 'b;
                }
            }
        }
    }
}

/// Compile-only. A `loop_state` loop followed by a second independent loop
/// with the same label `'a`. Verifies that the transform does not conflate
/// the two `'a` scopes and handles each loop's `continue 'a` independently.
#[cps_fn]
#[allow(dead_code, unreachable_code)]
fn two_independent_same_label_loops() {
    let mut env = String::from("outer");
    #[colorless::loop_state(env)]
    'a: loop {
        loop {
            if true {
                continue 'a;
            } else {
                break 'a;
            }
        }
    }

    // Second independent 'a — different scope, same spelling.
    'a: loop {
        loop {
            continue 'a;
        }
    }
}

/// Compile-only. An `if false { continue }` path inside a deeply nested loop
/// stack where `continue 'a` and `break 'a` appear in different inner loops.
/// The `return` from inside a loop-as-expression is the exotic feature under test.
#[cps_fn]
#[allow(dead_code, unreachable_code, clippy::never_loop)]
fn loop_with_return_inside_loop_expr() {
    'a: loop {
        loop {
            if false {
                continue;
            }
            loop {
                if true {
                    continue 'a;
                }
                if true {
                    break 'a;
                }
                break;
            }

            loop {
                // `return EXPR` where EXPR is itself a loop expression.
                return 'l: loop {
                    continue 'a;
                };
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Runtime checks: exotic syntax that terminates
// ─────────────────────────────────────────────────────────────────────────────

/// A four-level `break 'outermost` from the innermost loop terminates immediately.
#[test]
fn deep_break_to_outermost_terminates() {
    #[cps_fn]
    fn run() {
        'a: loop {
            loop {
                loop {
                    loop {
                        break 'a;
                    }
                }
            }
        }
    }

    run();
}

/// `while let Some(x) = LOOP_EXPR { … }` where the loop expression itself
/// executes `break 'while_label` — the outer `while let` exits via the label
/// before its body ever runs.
#[test]
fn loop_expr_as_while_let_scrutinee_breaks_via_label() {
    #[cps_fn]
    #[allow(unreachable_code)]
    fn run() {
        let s = "S".to_string();
        'w: while let Some(_x) = loop {
            if true {
                break 'w;
            }
            break Some(s);
        } {
            loop {} // body never reached
        }
        println!("completed");
    }

    run();
}

/// A loop expression used as a `let` RHS, followed by a second plain loop.
/// Both loops break immediately.
#[test]
fn loop_expr_as_let_rhs_followed_by_another_loop() {
    #[cps_fn]
    fn run() {
        let s = "S".to_string();
        loop {
            let _x = loop {
                if true {
                    break s;
                }
            };
            loop {
                break;
            }
            break;
        }
    }

    run();
}

/// Tuple `let-else` where the tuple contains a diverging `if` arm in its first
/// element and an `Option` pattern in its second. `emit_inert = true` is set
/// to also exercise the inert validation path.
#[test]
fn tuple_let_else_with_diverging_first_element() {
    #[cps_fn(emit_inert = true)]
    fn run() -> i32 {
        let (_, Some(val)) = (
            if 1 == 2 {
                return 2;
            } else {
                1
            },
            Some(1),
        ) else {
            return -1;
        };
        fn inner(v: i32) -> i32 {
            v
        }
        inner(val) + 10
    }

    // 1 != 2 → first element is 1; second is Some(1) → val = 1 → 1 + 10 = 11
    assert_eq!(run(), 11);
}

/// A `while` loop with a mutable reference argument accumulates correctly.
#[test]
fn while_loop_with_mut_ref_argument() {
    fn fetch(id: i32) -> i32 {
        id
    }

    #[cps_fn]
    fn run(x: &mut i32) -> i32 {
        let mut i = 0;
        while i < 10 {
            *x += fetch(i);
            i += 1;
        }
        *x
    }

    // sum(0..10) = 45
    let mut acc = 0;
    assert_eq!(run(&mut acc), 45);
}

/// `let-else` inside a `loop` where the else arm does `return` (exits function).
/// When the value is always `None`, the loop exits via `return` on the first
/// iteration. `emit_inert = true` validates the inert path too.
#[test]
fn let_else_in_loop_returns_on_mismatch() {
    #[cps_fn(emit_inert = true)]
    fn run() -> bool {
        fn get() -> Option<i32> {
            None
        }
        loop {
            let Some(_v) = get() else {
                return false;
            };
            println!("{_v}");
        }
    }

    assert!(!run());
}

/// `return LOOP_EXPR` where the loop expression evaluates to its break value.
#[test]
fn return_of_loop_expression_value() {
    #[cps_fn]
    fn run() -> i32 {
        loop {
            return loop {
                break 3;
            };
        }
    }

    assert_eq!(run(), 3);
}

/// Same as above, but the loop expression is first bound to a local, then
/// that local is `return`ed.
#[test]
fn return_of_loop_expression_via_let_binding() {
    #[cps_fn]
    fn run() -> i32 {
        let l = loop {
            let inner = loop {
                break 3;
            };
            return inner;
        };
        l
    }

    assert_eq!(run(), 3);
}

/// A labeled block inside a loop — `break 'n VALUE` exits the block, not the loop.
/// The loop itself then uses the block's value and breaks.
#[test]
fn labeled_block_inside_loop_as_local_value() {
    #[cps_fn]
    fn run() -> i32 {
        let a = 'a: loop {
            let inner = 'n: {
                if true {
                    break 'n 1;
                }
                2
            };
            break 'a inner;
        };
        a
    }

    assert_eq!(run(), 1);
}

/// `break 'outer VALUE` from the inner of two nested labeled loops,
/// choosing between the two exits at compile-time via a constant `if`.
#[test]
fn nested_labeled_loop_break_with_value() {
    #[cps_fn]
    fn run() -> i32 {
        'a: loop {
            'b: loop {
                if false {
                    break 'b 1;
                } else {
                    break 'a 2;
                }
            }
        }
    }

    assert_eq!(run(), 2);
}

/// `?` inside a `loop_state` for-loop: each iteration applies `?` to the
/// result of a function. All iterations succeed here, so the full loop runs.
#[test]
fn try_operator_inside_loop_state_for_loop_succeeds() {
    fn passthrough(c: String) -> Result<String, i32> {
        Ok(c)
    }

    #[cps_fn]
    fn run(start: String) -> Result<String, i32> {
        let mut c = start;
        #[colorless::loop_state(pub c)]
        for _ in 0..5 {
            c = passthrough(c)?;
        }
        Ok(c)
    }

    assert_eq!(run(String::from("hello")), Ok(String::from("hello")));
}
