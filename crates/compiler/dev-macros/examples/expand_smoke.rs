use dev_macros::{cps_expr, cps_fn};
use std::marker::PhantomData;

fn unify_value<T>(a: T, _b: PhantomData<T>) -> T {
    a
}

#[cps_fn]
fn lowered_function(input: i32) -> i32 {
    let doubled = input * 2;
    doubled + 1
}

#[cps_fn]
fn lowered_branch(input: i32) -> i32 {
    let adjusted = if input > 0 { input + 1 } else { input - 1 };
    adjusted * 2
}

fn main() {
    let value = cps_expr!({
        let base = 20;
        base + 2
    });

    assert_eq!(value, 22);
    assert_eq!(lowered_function(20), 41);
    assert_eq!(lowered_branch(4), 10);
    assert_eq!(lowered_branch(-4), -10);
}
