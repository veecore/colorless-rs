#![cfg(feature = "relaxed-drop")]

use colorless::{
    Async, AsyncExt, Color, Colored, Colorize, IdentityExt, color, colorless,
    test_util::block_on_test,
};
use colorless_core::ready;
use core::future::poll_fn;
use core::task::Poll;
use std::fmt::Debug;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[allow(dead_code)]
#[colorless(C)]
trait Foo: Debug {
    fn foo(&self) -> Result<(), ()> {
        let d = ready(Ok(40)).resolve? + 2;
        println!("{self:#?}{}", d);
        Ok(())
    }
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

        #[loop_state(pub mut total, pub mut output)]
        #[default_break_state]
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

    fn suspend_once<T>(value: T) -> impl Colored<Async, Output = T> {
        <_ as Colorize<Async>>::colorize(async move {
            yield_once().await;
            value
        })
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
