#![cfg(feature = "relaxed-drop")]

use colorless::{
    Async, AsyncExt, Color, Colored, IdentityExt, color, colorless, test_util::block_on_test,
};
use colorless_core::ready;
use colorless_core::test_util::block_on_local_test;
use core::future::poll_fn;
use core::task::Poll;
use std::alloc::Layout;
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

// some of the hallmark tests from archive ( kinda weak now )

#[test]
fn archived() {
    #[colorless]
    fn ex9_c() -> i32 {
        let mut x = 0;
        pin!(x);
        let y = rb!(x);
        let mut i = 0;

        borrow!(y);
        slot!(y);
        #[state(mut i, pub mut y)]
        #[default_state]
        while i < 10 {
            *rb!(y) += ready(i).resolve;
            i += 1;
        }
        take_slot!(y);
        unborrow!(y);
        *y
    }

    // ---------------------------------------------------------
    // Example 1: Simple linear sequence
    // ---------------------------------------------------------
    #[colorless]
    fn ex1() -> i32 {
        let a = ready(1).resolve;
        let b = ready(a).resolve;
        a + b
    }

    // ---------------------------------------------------------
    // Example 2: Branching (if / else)
    // ---------------------------------------------------------
    #[colorless]
    fn ex2(flag: bool) -> i32 {
        if flag {
            ready(1).resolve
        } else {
            ready(2).resolve
        }
    }

    // ---------------------------------------------------------
    // Example 3: Branching with unequal arms (Pure vs Colored)
    // ---------------------------------------------------------
    #[colorless]
    fn ex3(flag: bool) -> i32 {
        if flag { ready(1).resolve } else { 100 }
    }

    // ---------------------------------------------------------
    // Example 4: Simple loop without state
    // ---------------------------------------------------------
    #[colorless]
    fn ex4() {
        loop {
            ready(1).resolve;
        }
    }

    // ---------------------------------------------------------
    // Example 5: Loop with outer state mutation
    // ---------------------------------------------------------
    #[colorless]
    fn ex5() -> i32 {
        let mut sum = 0;
        let mut i = 0;
        #[state(pub mut sum, mut i)]
        while i < 10 {
            sum += ready(i).resolve;
            i += 1;
        }
        sum
    }

    assert_eq!(ex5().sync_(), (9 * (9 + 1)) / 2);

    // ---------------------------------------------------------
    // Example 6: Loop with early break
    // ---------------------------------------------------------
    #[colorless]
    fn ex6() -> i32 {
        let mut i = 0;
        #[state(mut i)]
        loop {
            let x = ready(i).resolve;
            if x > 50 {
                break x;
            }
            i += 1;
        }
    }

    // ---------------------------------------------------------
    // Example 7: Loop with early continue
    // ---------------------------------------------------------
    #[colorless]
    fn ex7() -> i32 {
        let mut sum = 0;
        let mut i = 0;
        #[state(mut i, pub mut sum)]
        while i < 10 {
            i += 1;
            if i % 2 == 0 {
                continue;
            }
            sum += ready(i).resolve;
        }
        sum
    }

    // ---------------------------------------------------------
    // Example 8: Mutating an external reference (&mut T)
    // ---------------------------------------------------------
    #[colorless]
    fn ex8(x: &mut i32) -> i32 {
        let mut i = 0;
        borrow!(x);
        slot!(x);
        #[state(mut i, pub mut x)]
        #[default_state]
        while i < 10 {
            *rb!(x) += ready(i).resolve;
            i += 1;
        }
        take_slot!(x);
        unborrow!(x);
        *x
    }

    // ------------------------------------------------
    // Example 9: Self-Referential Locals across Yields
    // ------------------------------------------------
    #[colorless]
    fn ex9() -> i32 {
        let mut x = 0;
        pin!(x);
        let y = rb!(x);
        let a = ready(1).resolve;
        *y += a;
        *y
    }

    // ---------------------------------------------------------
    // Example 10: match Statement
    // ---------------------------------------------------------
    #[colorless]
    fn ex10(x: i32) -> i32 {
        match x {
            0 => ready(0).resolve,
            1 => 100,
            _ => ready(2).resolve,
        }
    }

    // ---------------------------------------------------------
    // Example 11: Nested Loops
    // ---------------------------------------------------------
    #[colorless]
    fn ex11() -> i32 {
        let mut sum = 0;
        #[state(pub mut sum)]
        for i in 0..5 {
            #[state(pub mut sum, i)]
            for j in 0..5 {
                sum += ready(i + j).resolve;
            }
        }
        sum
    }

    // ---------------------------------------------------------
    // Example 12: while let Loop
    // ---------------------------------------------------------
    #[colorless]
    fn ex12(mut opt: Option<i32>) {
        #[state(mut opt)]
        while let Some(x) = opt {
            opt = if x < 10 { Some(ready(x).resolve) } else { None };
        }
    }

    // ---------------------------------------------------------
    // Example 13: for Loop
    // ---------------------------------------------------------
    #[colorless]
    fn ex13(vec: Vec<i32>) -> i32 {
        let mut sum = 0;
        #[state(pub mut sum)]
        for x in vec {
            sum += ready(x).resolve;
        }
        sum
    }

    // ------------------------
    // Example 14: Early return
    // ------------------------
    #[colorless]
    fn ex14() -> i32 {
        let x = ready(1).resolve;
        if x == 0 {
            return 0;
        }
        ready(2).resolve
    }

    // ---------------------------
    // Example 15: The ? operator
    // ---------------------------
    #[colorless]
    fn ex15() -> Result<i32, String> {
        // Assuming fetch_res returns impl Colored<C, Output = Result<i32, String>>
        let x = ready(Ok::<_, String>(1)).resolve?;
        Ok(x)
    }

    // ---------------------------------------------------------
    // Example 16: Closures containing
    // ---------------------------------------------------------
    // Modified for another test
    #[colorless(Z)]
    fn ex16() {
        let f = || {
            color! {
                #![colorless(color = Z)]

                ready(1).resolve
            }
        };
        f().resolve;
    }

    #[colorless]
    fn ex17() -> i32 {
        let x = 1;
        let x = ready(x + 1).resolve;
        x
    }

    assert_eq!(ex17().sync_(), 2);

    // ---------------------------------------------------------
    // Example 19: Labeled loop breaks
    // ---------------------------------------------------------
    #[cfg(any())]
    // fixme: general loop issue... this should work though.
    // if we roll into a while loop with iter_0 as pub, it does
    #[colorless]
    fn ex19() -> i32 {
        ready(()).resolve;
        let mut sum = 0;
        'outer: for i in 0..5 {
            #[no_shadow]
            for j in 0..5 {
                if j == 2 {
                    break 'outer;
                }
            }
        }
        sum
    }

    // ----------------------
    // Example 20: Drop Order
    // ----------------------
    use std::sync::Mutex;
    #[colorless]
    fn ex20(m: &Mutex<i32>) {
        let _guard = m.lock().unwrap();
        ready(1).resolve;
        keep_alive!(_guard);
    }

    // ----------------------
    // Example 21: Send-ness
    // ----------------------
    #[colorless]
    fn ex21() -> i32 {
        let rc = std::rc::Rc::new(10); // !Send
        let x = *rc;
        // `rc` is dead here in standard async rust, so this future SHOULD be `Send`.
        let a = ready(x).resolve;
        a
    }

    fn assert_send<T: Send>(_t: &T) {}

    assert_send(&ex21().async_());

    // ---------------------------------------------------------
    // Example 23: Pattern Matching with `ref` bindings across yields
    // ---------------------------------------------------------
    #[allow(dead_code)]
    pub enum MyEnum {
        Data(String),
        None,
    }

    // fixme
    #[colorless]
    fn ex23(mut e: MyEnum) {
        pin!(e); // this is inconvenient when other arms consume
        match rb!(e) {
            // pin should be around here
            MyEnum::Data(/*ref */ s) => {
                ready(1).resolve;
                println!("{}", s);
            }
            _ => {}
        }
    }

    #[colorless]
    fn ex23_take_2(e: MyEnum) {
        match e {
            MyEnum::Data(mut s) => {
                pin!(s);
                ready(1).resolve;
                println!("{}", rb!(s));
            }
            _ => {}
        }
    }

    // ---------------------------------------------------------
    // Example 40: Stack Bloat (Closure Environment Size)
    // ---------------------------------------------------------
    #[colorless]
    fn ex40() {
        let large = [0u8; 10_000];
        ready(1).resolve;
        println!("{}", large[0]);
        let large2 = [0u8; 10_000];
        ready(2).resolve;
        println!("{}", large2[0]);
    }

    let fut = ex40().async_();
    // large is carried on
    assert_eq!(size_of_val(&fut), 10_000 + size_of::<usize>());

    // ---------------------------------------------------------
    // Example 50: `break` to multiple outer labels
    // ---------------------------------------------------------
    #[colorless]
    fn ex50(mut v: bool) {
        #[state(pub mut v)]
        'outer: loop {
            #[no_shadow]
            'inner: loop {
                v = ready(v).resolve;
                if v == true {
                    break 'inner;
                }
                if v == false {
                    break 'outer;
                }
            }
            assert_eq!(v, true);
            return;
        }
        assert_eq!(v, false);
    }

    for t in [true, false] {
        ex50(t).sync_();
        block_on_test(ex50(t).async_());
    }
}

#[test]
fn match_pat_influence() {
    // we don't target pat
    #[allow(unreachable_code, unused_variables)]
    #[colorless]
    fn match_pat_influence() -> Result<(), ()> {
        ready(()).resolve;
        let x = match 1 {
            _ if ready(Err::<i32, _>(())).sync_()? > 0 => unreachable!(),
            _ => unreachable!(),
        };
        unreachable!();
        x
    }

    match_pat_influence().sync_().unwrap_err();
}

#[test]
fn recursion() {
    // ---------------------------------------------------------
    // Example 22: Recursive Functions (Infinite Types)
    // ---------------------------------------------------------
    #[colorless(erase)]
    fn ex22(n: i32) -> i32 {
        if n == 0 {
            return 0;
        }
        n + ex22(n - 1).resolve
    }

    assert_eq!(ex22(5).sync_(), (5 * (5 + 1)) / 2);
    // dyn throws the send out the window
    assert_eq!(block_on_local_test(ex22(5).async_()), (5 * (5 + 1)) / 2);
}
