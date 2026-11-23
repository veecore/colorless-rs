#![cfg(feature = "relaxed-drop")]

use colorless::{
    Async, AsyncExt, Color, Colored, IdentityExt, color, colorless, test_util::block_on_test,
};
use colorless_core::ready;
use core::future::poll_fn;
use core::task::Poll;
use std::cell::Cell;
use std::fmt::Debug;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

macro_rules! method {
    () => {
        #[colorless(C as)]
        fn bar(&self) {
            let _r = self.foo().resolve;
        }
    };
}

#[allow(dead_code)]
#[colorless(C)]
trait Foo: Debug {
    fn foo(&self) -> Result<(), ()> {
        let d = ready(Ok(40)).resolve? + 2;
        println!("{self:#?}{}", d);
        Ok(())
    }

    method! {}
}

#[colorless(color = Async)]
impl Foo for () {
    #[inline]
    async fn foo(&self) -> Result<(), ()> {
        Ok(())
    }
}

#[colorless]
fn add_one(value: i32) -> i32 {
    value + 1
}

#[colorless]
fn add_chained(left: i32, right: i32) -> i32 {
    let left = add_one(left).resolve;
    let right = add_one(right).resolve;
    left + right
}

#[colorless]
fn choose(value: i32) -> i32 {
    if value > 0 { add_one(value).resolve } else { 0 }
}

#[test]
fn flattened_macro_shape_is_well_typed() {
    #[colorless]
    fn run(values: Vec<usize>) -> Result<(usize, Vec<usize>), ()> {
        let mut total = 0_usize;
        let mut output = Vec::new();

        #[state(pub mut total, pub mut output)]
        #[default_state]
        for value in values {
            if value == 0 {
                return Err(());
            }
            let stepped = ready(Ok(value)).resolve?;
            total += stepped * 2;
            ready(value).resolve;
            output.push(stepped);
        }
        Ok((total, output))
    }

    let f = run(vec![1, 2, 3]).async_();
    block_on_test(f).unwrap();
}

#[colorless]
fn linear_after_resolve(mut value: usize) -> usize {
    value = ready(value).resolve;
    if value == 0 {
        return value;
    }
    value = ready(value).resolve;
    if value == 1 {
        return value;
    }
    value = ready(value).resolve;
    if value == 2 {
        return value;
    }
    value = ready(value).resolve;
    if value == 3 {
        return value;
    }
    value = ready(value).resolve;
    if value == 4 {
        return value;
    }
    value
}

#[colorless]
fn branching_across_resolve(mut value: usize) -> usize {
    if value & 1 == 0 {
        value = ready(value).resolve;
    } else {
        value = ready(value + 10).resolve;
    }
    if value & 2 == 0 {
        value = ready(value).resolve;
    } else {
        value = ready(value + 20).resolve;
    }
    if value & 4 == 0 {
        value = ready(value).resolve;
    } else {
        value = ready(value + 40).resolve;
    }
    if value & 8 == 0 {
        value = ready(value).resolve;
    } else {
        value = ready(value + 80).resolve;
    }
    if value & 16 == 0 {
        value = ready(value).resolve;
    } else {
        value = ready(value + 160).resolve;
    }
    if value & 32 == 0 {
        value = ready(value).resolve;
    } else {
        value = ready(value + 320).resolve;
    }
    value
}

#[test]
fn attribute_macro_has_equivalent_identity_and_async_results() {
    fn step<C: Color>(value: i32) -> impl Colored<C, Output = i32> {
        C::wrap(move || value + 1)
    }

    #[colorless]
    fn checked(value: i32) -> Result<i32, &'static str> {
        (value >= 0).then_some(value).ok_or("negative")
    }

    /// Exercises sequential binds, match arms, early returns, and `?` after the
    /// actual procedural macro has rewritten the function.
    #[colorless]
    fn generated_control_flow(value: i32) -> Result<i32, &'static str> {
        let first = step(value).resolve;
        let selected = match first.rem_euclid(3) {
            0 => step(first).resolve,
            1 => first * 2,
            _ => return Err("reserved"),
        };

        if selected > 12 {
            return Err("large");
        }

        Ok(checked(selected).resolve?)
    }

    for (input, expected) in [
        (0, Ok(2)),
        (1, Err("reserved")),
        (2, Ok(4)),
        (11, Err("large")),
    ] {
        assert_eq!(generated_control_flow(input).sync_(), expected);
        assert_eq!(
            block_on_test(generated_control_flow(input).async_()),
            expected
        );
    }
}

#[test]
fn inline_macro_is_lazy_until_its_async_computation_is_polled() {
    async fn yield_once() {
        let mut yielded = false;
        poll_fn(move |context| {
            if yielded {
                Poll::Ready(())
            } else {
                yielded = true;
                context.waker().wake_by_ref();
                Poll::Pending
            }
        })
        .await;
    }

    #[colorless(color = Async)]
    async fn suspend_once<T>(value: T) -> T {
        yield_once().await;
        value
    }

    let calls = Arc::new(AtomicUsize::new(0));
    let observed = calls.clone();
    let computation = color! {
        let value = suspend_once(40).resolve;
        observed.fetch_add(1, Ordering::SeqCst);
        value + 2
    };

    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(block_on_test(computation.async_()), 42);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

// ── allow_macros integration tests ───────────────────────────────────────────

/// A macro that appends to a shared log to prove it actually ran.
macro_rules! push_log {
    ($log:expr, $msg:literal) => {
        $log.push_str($msg);
    };
}

#[test]
fn allow_macros_targeted_permits_named_macro_across_resolve() {
    use colorless_core::ready;

    #[colorless(allow_macros(push_log))]
    fn logged_add(log: &mut String, a: i32, b: i32) -> i32 {
        push_log!(log, "before-resolve ");
        let sum = ready(a + b).resolve;
        push_log!(log, "after-resolve");
        sum
    }

    let mut log = String::new();
    let result = logged_add(&mut log, 3, 4).sync_();
    assert_eq!(result, 7);
    assert_eq!(log, "before-resolve after-resolve");
}

macro_rules! custom_assert {
    ($($tt:tt)*) => {
        assert!($($tt)*);
    };
}

macro_rules! call {
    () => {};
}

#[test]
fn allow_macros_all_permits_any_macro_across_resolve() {
    use colorless_core::ready;

    // Uses the Cell as a cross-resolve observable side effect without &mut.
    #[colorless(allow_macros(all))]
    fn counted(counter: &Cell<u32>, value: i32) -> i32 {
        // macro call before the resolve point
        custom_assert!(value >= 0, "value must be non-negative");
        let stepped = ready(value + 1).resolve;
        // macro call after the resolve point
        call!();
        counter.set(counter.get() + 1);
        stepped
    }

    let counter = Cell::new(0u32);
    assert_eq!(counted(&counter, 10).sync_(), 11);
    assert_eq!(counter.get(), 1);
}

macro_rules! record_step {
    ($cell:expr, $val:expr) => {
        $cell.set($cell.get() + $val);
    };
}

#[test]
fn allow_macros_inside_color_block() {
    use colorless_core::ready;

    let counter = Cell::new(0u32);
    let counter_ref = &counter;
    let computation = color! {
        #![colorless(color = colorless::Identity, allow_macros(record_step))]

        record_step!(counter_ref, 10);
        let v = ready(42_i32).resolve;
        record_step!(counter_ref, 5);
        v
    };

    let result = computation.sync_();
    assert_eq!(result, 42);
    assert_eq!(counter.get(), 15);
}

#[test]
#[should_panic(expected = "not implemented")]
fn dev_mode() {
    #[colorless(out_of_my_way)]
    fn run(left: i32, right: i32) -> i32 {
        let left = add_one(left).resolve;
        let right = add_one(right).resolve;
        left + right
    }

    run(0, 0).sync_();
}
