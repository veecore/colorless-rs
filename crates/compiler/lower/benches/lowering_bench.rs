//! CPS Lowering Throughput & Latency Benchmark Suite.
//!
//! ## Benchmark Groups
//!
//! | Group | What it stresses |
//! |-------|--------------------|
//! | A. Flat sequences | Baseline; pure let-binding chains, no control flow |
//! | B. Branching growth | How cost scales as branch-count / nesting depth grows |
//! | C. Loop desugaring | Stack-signal cost: for/while/loop bodies with break/continue/return |
//! | D. Stack-signal pressure | Deeply nested loops forcing ancestor-signal propagation |
//! | E. Try (`?`) inside loops | The ? + ret_sig interaction under varying loop depth |
//! | F. Match arm explosion | Many arms with guards and early returns |
//! | G. ANF hoisting storm | Tuples/arrays/calls with hoistable sub-expressions at every position |
//! | H. End-to-end pipeline | lower + quote: wall-clock cost per function |
//!
//! Each benchmark uses `iter_batched` so AST cloning (setup) is excluded from
//! the measured window.  Only the lowering pass itself is timed.

use colorless_lower::cps::CpsStep;
use colorless_lower::ir::{CpsBlock, IntoSyn};
use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use syn::{Block, Expr, ItemFn, Stmt, parse_quote};

#[inline(never)]
fn lower_block(block: Block) -> Block {
    let block: CpsBlock = block.step().unwrap();
    block.into_syn()
}

#[inline(never)]
fn lower_fn(func: ItemFn) -> ItemFn {
    let mut func = func;
    let block: CpsBlock = func.block.step().unwrap();
    let block: Block = block.into_syn();
    func.block = block.into();
    func
}

// ============================================================================
// A. FLAT SEQUENCES — baseline / scaling check
// ============================================================================
fn bench_a_flat_sequences(c: &mut Criterion) {
    let mut group = c.benchmark_group("A. Flat Sequences");

    for n in [4usize, 8, 16, 32, 64] {
        let stmts: Vec<Stmt> = (0..n)
            .map(|i| {
                let src = if i == 0 {
                    format!("let __v{i} = fetch_{i}(seed);")
                } else {
                    format!("let __v{i} = __v{} + {i};", i - 1)
                };
                syn::parse_str::<Stmt>(&src).unwrap()
            })
            .collect();
        let tail: Expr = parse_quote!(__v0 + 0);
        let block = Block {
            brace_token: Default::default(),
            stmts: {
                let mut s = stmts;
                s.push(Stmt::Expr(tail, None));
                s
            },
        };
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::new("let-chain", n), &block, |b, blk| {
            b.iter_batched(
                || blk.clone(),
                |blk| black_box(lower_block(blk)),
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

// ============================================================================
// B. BRANCHING GROWTH — how cost scales with depth / fanout
// ============================================================================
fn bench_b_branching_growth(c: &mut Criterion) {
    let mut group = c.benchmark_group("B. Branching Growth");

    // B1: Cascading early-return guards (linear chain of ifs)
    for n_guards in [2usize, 4, 8, 12, 16] {
        let mut stmts: Vec<String> = Vec::new();
        for i in 0..n_guards {
            stmts.push(format!("let __g{i} = fetch_{i}(seed);"));
            stmts.push(format!("if __g{i} < 0 {{ return -{i}i32; }}"));
        }
        stmts.push("seed".to_string());
        let src = format!("{{ {} }}", stmts.join("\n"));
        let block = syn::parse_str::<Block>(&src).unwrap();
        group.throughput(Throughput::Elements(n_guards as u64));
        group.bench_with_input(
            BenchmarkId::new("cascading-if-returns", n_guards),
            &block,
            |b, blk| {
                b.iter_batched(
                    || blk.clone(),
                    |blk| black_box(lower_block(blk)),
                    BatchSize::SmallInput,
                )
            },
        );
    }

    // B2: Binary nesting — 2^depth leaf continuations
    fn nested_if(depth: usize, cur: usize) -> String {
        if depth == 0 {
            return format!("val_{cur}");
        }
        format!(
            "if cond_{cur} {{ {} }} else {{ {} }}",
            nested_if(depth - 1, cur * 2),
            nested_if(depth - 1, cur * 2 + 1)
        )
    }
    for depth in [2usize, 3, 4, 5] {
        let src = format!("{{ {} }}", nested_if(depth, 0));
        let block = syn::parse_str::<Block>(&src).unwrap();
        let leaves = 1usize << depth;
        group.throughput(Throughput::Elements(leaves as u64));
        group.bench_with_input(
            BenchmarkId::new("binary-nested-if", depth),
            &block,
            |b, blk| {
                b.iter_batched(
                    || blk.clone(),
                    |blk| black_box(lower_block(blk)),
                    BatchSize::SmallInput,
                )
            },
        );
    }

    // B3: If with return in both arms (all paths diverge)
    let diverge_both: Block = parse_quote! {
        {
            let a = fetch_a(x);
            let b = fetch_b(y);
            let c = if a > 0 {
                if b > 0 { return 1; } else { return 2; }
            } else {
                if b > 0 { return 3; } else { return 4; }
            };
            c + 10
        }
    };
    group.bench_function("divergent-if-both-arms-4-guards", |b| {
        b.iter_batched(
            || diverge_both.clone(),
            |blk| black_box(lower_block(blk)),
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

// ============================================================================
// C. LOOP DESUGARING — per-loop-kind stack signal cost
// ============================================================================
fn bench_c_loop_desugaring(c: &mut Criterion) {
    let mut group = c.benchmark_group("C. Loop Desugaring");

    // C1: for loop — plain iteration with continue + break (no ret_sig)
    let for_plain: Block = parse_quote! {
        {
            let mut sum = 0i32;
            for x in items {
                if x < 0 { continue; }
                if sum > 10_000i32 { break; }
                sum += x;
            }
            sum
        }
    };
    group.bench_function("for-plain-continue-break", |b| {
        b.iter_batched(
            || for_plain.clone(),
            |blk| black_box(lower_block(blk)),
            BatchSize::SmallInput,
        )
    });

    // C2: for loop — early return (triggers ret_sig allocation)
    let for_early_return: Block = parse_quote! {
        {
            let mut sum = 0i32;
            for x in items {
                if x == 0 { return -1; }
                sum += x;
            }
            sum
        }
    };
    group.bench_function("for-early-return-ret-sig", |b| {
        b.iter_batched(
            || for_early_return.clone(),
            |blk| black_box(lower_block(blk)),
            BatchSize::SmallInput,
        )
    });

    // C3: while loop — condition + body + continue + early return
    let while_block: Block = parse_quote! {
        {
            let mut count = 0i32;
            let mut acc = 0i32;
            while count < limit {
                if count == abort_at { return -999; }
                if count % 2 == 0 { count += 1; continue; }
                acc += count;
                count += 1;
            }
            acc
        }
    };
    group.bench_function("while-continue-return", |b| {
        b.iter_batched(
            || while_block.clone(),
            |blk| black_box(lower_block(blk)),
            BatchSize::SmallInput,
        )
    });

    // C4: while-let loop with early return
    let while_let_block: Block = parse_quote! {
        {
            let mut total = 0i32;
            while let Some(val) = items.pop() {
                if val < 0 { return Err("negative"); }
                total += val;
            }
            Ok(total)
        }
    };
    group.bench_function("while-let-with-return", |b| {
        b.iter_batched(
            || while_let_block.clone(),
            |blk| black_box(lower_block(blk)),
            BatchSize::SmallInput,
        )
    });

    // C5: loop { break val } — value-carrying break
    let loop_val_break: Block = parse_quote! {
        {
            let result = loop {
                let v = next_val();
                if v > threshold { break v * 2; }
                if v < 0 { break 0; }
            };
            result + 1
        }
    };
    group.bench_function("loop-value-break", |b| {
        b.iter_batched(
            || loop_val_break.clone(),
            |blk| black_box(lower_block(blk)),
            BatchSize::SmallInput,
        )
    });

    // C6: loop with all three signals: break-with-val, continue, return
    let loop_all_signals: Block = parse_quote! {
        {
            let result = loop {
                let v = next_val();
                if v == 0 { return -1; }
                if v < 0 { continue; }
                if v > threshold { break v; }
            };
            result
        }
    };
    group.bench_function("loop-all-three-signals", |b| {
        b.iter_batched(
            || loop_all_signals.clone(),
            |blk| black_box(lower_block(blk)),
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

// ============================================================================
// D. STACK-SIGNAL PRESSURE — nested loops driving ancestor signal checks
// ============================================================================
fn bench_d_stack_signal_pressure(c: &mut Criterion) {
    let mut group = c.benchmark_group("D. Stack-Signal Pressure");

    // D1: 2-level nested loops — inner return propagates up through outer
    let nested_2: Block = parse_quote! {
        {
            'outer: for a in outer_items {
                for b in inner_items {
                    if a == 0 && b == 0 { return -1; }
                    if a + b > limit { break 'outer; }
                }
            }
            done()
        }
    };
    group.bench_function("nested-2-level-cross-break-return", |b| {
        b.iter_batched(
            || nested_2.clone(),
            |blk| black_box(lower_block(blk)),
            BatchSize::SmallInput,
        )
    });

    // D2: 3-level nested loops — worst-case ancestor-signal check chain
    let nested_3: Block = parse_quote! {
        {
            let mut found = false;
            'outer: for a in 0..n {
                'mid: for b in 0..n {
                    for c in 0..n {
                        if a == 0 && b == 0 && c == 0 { return -999; }
                        if a * b * c > threshold { break 'outer; }
                        if b == c { break 'mid; }
                        if c % 2 == 0 { continue; }
                        found = true;
                    }
                }
            }
            found
        }
    };
    group.bench_function("nested-3-level-full-signal-chain", |b| {
        b.iter_batched(
            || nested_3.clone(),
            |blk| black_box(lower_block(blk)),
            BatchSize::SmallInput,
        )
    });

    // D3: Labeled loop as expression — break-with-value from deep inside
    let labeled_loop_expr: Block = parse_quote! {
        {
            let score = 'search: loop {
                let v = next_candidate();
                'inner: for x in v.items() {
                    if x.is_valid() {
                        if x.score() > high_mark { break 'search x.score(); }
                        if x.score() < low_mark { break 'inner; }
                    }
                }
                if v.exhausted() { break 'search 0; }
            };
            score * 2
        }
    };
    group.bench_function("labeled-loop-expr-deep-break-value", |b| {
        b.iter_batched(
            || labeled_loop_expr.clone(),
            |blk| black_box(lower_block(blk)),
            BatchSize::SmallInput,
        )
    });

    // D4: for loop containing a while loop — cross-loop return signal
    let for_contains_while: Block = parse_quote! {
        {
            for chunk in chunks {
                let mut i = 0;
                while i < chunk.len() {
                    if chunk[i] < 0 { return Err("negative in chunk"); }
                    i += 1;
                }
            }
            Ok(())
        }
    };
    group.bench_function("for-containing-while-cross-return", |b| {
        b.iter_batched(
            || for_contains_while.clone(),
            |blk| black_box(lower_block(blk)),
            BatchSize::SmallInput,
        )
    });

    // D5: 4-level nesting — maximal signal propagation depth
    let nested_4: Block = parse_quote! {
        {
            'l1: for a in 0..n {
                'l2: for b in 0..n {
                    'l3: for c in 0..n {
                        for d in 0..n {
                            if a+b+c+d == 0 { return -1; }
                            if d > limit { break 'l3; }
                            if c > limit { break 'l2; }
                            if a+b+c+d > big_limit { break 'l1; }
                        }
                    }
                }
            }
            0
        }
    };
    group.bench_function("nested-4-level-maximal-signal-depth", |b| {
        b.iter_batched(
            || nested_4.clone(),
            |blk| black_box(lower_block(blk)),
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

// ============================================================================
// E. TRY (`?`) INSIDE LOOPS — ret_sig allocation + signal interaction
// ============================================================================
fn bench_e_try_in_loops(c: &mut Criterion) {
    let mut group = c.benchmark_group("E. Try-in-Loop Stack Signals");

    // E1: Single ? in for loop
    let try_for_single: Block = parse_quote! {
        {
            let mut acc = 0i32;
            for x in items {
                let v = parse_item(x)?;
                acc += v;
            }
            Ok(acc)
        }
    };
    group.bench_function("single-try-in-for", |b| {
        b.iter_batched(
            || try_for_single.clone(),
            |blk| black_box(lower_block(blk)),
            BatchSize::SmallInput,
        )
    });

    // E2: Multiple ? operators in one for loop body (shared ret_sig)
    let try_for_multi: Block = parse_quote! {
        {
            let mut acc = 0i32;
            for x in items {
                let raw = fetch(x)?;
                let parsed = parse(raw)?;
                let validated = validate(parsed)?;
                acc += validated;
            }
            Ok(acc)
        }
    };
    group.bench_function("multi-try-in-for-same-loop", |b| {
        b.iter_batched(
            || try_for_multi.clone(),
            |blk| black_box(lower_block(blk)),
            BatchSize::SmallInput,
        )
    });

    // E3: ? in while loop
    let try_while: Block = parse_quote! {
        {
            let mut count = 0i32;
            while count < limit {
                let v = get_next(count)?;
                count += v;
            }
            Ok(count)
        }
    };
    group.bench_function("try-in-while", |b| {
        b.iter_batched(
            || try_while.clone(),
            |blk| black_box(lower_block(blk)),
            BatchSize::SmallInput,
        )
    });

    // E4: ? in loop { break val } — combines ret_sig with break_val_sig
    let try_loop_break_val: Block = parse_quote! {
        {
            let result = loop {
                let v = fetch_next()?;
                if v > 0 { break v; }
            };
            Ok(result)
        }
    };
    group.bench_function("try-in-loop-with-break-value", |b| {
        b.iter_batched(
            || try_loop_break_val.clone(),
            |blk| black_box(lower_block(blk)),
            BatchSize::SmallInput,
        )
    });

    // E5: ? in nested loops (ret_sig propagates through 2 layers)
    let try_nested: Block = parse_quote! {
        {
            for chunk in chunks {
                for item in chunk {
                    let v = process(item)?;
                    if v == 0 { break; }
                }
            }
            Ok(())
        }
    };
    group.bench_function("try-in-nested-2-level", |b| {
        b.iter_batched(
            || try_nested.clone(),
            |blk| black_box(lower_block(blk)),
            BatchSize::SmallInput,
        )
    });

    // E6: ? plus explicit return in same loop (both signals allocated)
    let try_and_return: Block = parse_quote! {
        {
            for x in items {
                if x < 0 { return Err("negative direct"); }
                let v = parse(x)?;
                if v > limit { return Err("overflow"); }
            }
            Ok(())
        }
    };
    group.bench_function("try-and-return-same-loop", |b| {
        b.iter_batched(
            || try_and_return.clone(),
            |blk| black_box(lower_block(blk)),
            BatchSize::SmallInput,
        )
    });

    // E7: ? in for + explicit break + continue (full signal set in one loop)
    let try_full_signals: Block = parse_quote! {
        {
            let mut acc = 0i32;
            for x in items {
                if x < 0 { continue; }
                let v = parse(x)?;
                if v > cap { break; }
                acc += v;
            }
            Ok(acc)
        }
    };
    group.bench_function("try-continue-break-in-for", |b| {
        b.iter_batched(
            || try_full_signals.clone(),
            |blk| black_box(lower_block(blk)),
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

// ============================================================================
// F. MATCH ARM EXPLOSION — fanout cost in the lowering pass
// ============================================================================
fn bench_f_match_arms(c: &mut Criterion) {
    let mut group = c.benchmark_group("F. Match Arm Fanout");

    // F1: 4-arm match, no guards
    let match_4: Block = parse_quote! {
        {
            let r = match action {
                Action::Start => 1,
                Action::Stop  => 2,
                Action::Pause => 3,
                Action::Reset => 4,
            };
            r * 10
        }
    };
    group.bench_function("match-4-arm-no-guards", |b| {
        b.iter_batched(
            || match_4.clone(),
            |blk| black_box(lower_block(blk)),
            BatchSize::SmallInput,
        )
    });

    // F2: 8-arm match, guards + early returns
    let match_8_guards: Block = parse_quote! {
        {
            let r = match msg {
                Msg::A(x) if x > 100 => return 1,
                Msg::A(x)            => x + 1,
                Msg::B(y) if y < 0   => return -1,
                Msg::B(y)            => y * 2,
                Msg::C { a, b } if a == b => return 0,
                Msg::C { a, b }      => a + b,
                Msg::D(s)            => return s as i32,
                Msg::E               => 42,
            };
            r + 5
        }
    };
    group.bench_function("match-8-arm-guards-returns", |b| {
        b.iter_batched(
            || match_8_guards.clone(),
            |blk| black_box(lower_block(blk)),
            BatchSize::SmallInput,
        )
    });

    // F3: 12-arm literal match, alternating return/value arms
    let match_12: Block = parse_quote! {
        {
            let r = match code {
                0  => return "zero",
                1  => "one",
                2  => return "two",
                3  => "three",
                4  => return "four",
                5  => "five",
                6  => return "six",
                7  => "seven",
                8  => return "eight",
                9  => "nine",
                10 => return "ten",
                _  => "other",
            };
            r
        }
    };
    group.bench_function("match-12-arm-alternating-returns", |b| {
        b.iter_batched(
            || match_12.clone(),
            |blk| black_box(lower_block(blk)),
            BatchSize::SmallInput,
        )
    });

    // F4: Nested matches (match inside match arm body)
    let match_nested: Block = parse_quote! {
        {
            let outer = match outer_tag {
                Tag::X(inner) => match inner {
                    Inner::A => return 1,
                    Inner::B => 2,
                    Inner::C if flag => return 3,
                    Inner::C => 4,
                    _ => 0,
                },
                Tag::Y(v) if v > 0 => v * 10,
                Tag::Y(_)          => return -1,
                Tag::Z             => return 99,
            };
            outer + base
        }
    };
    group.bench_function("match-nested-2-level", |b| {
        b.iter_batched(
            || match_nested.clone(),
            |blk| black_box(lower_block(blk)),
            BatchSize::SmallInput,
        )
    });

    // F5: Match inside a for loop (arms with early returns from the function)
    let match_in_loop: Block = parse_quote! {
        {
            let mut total = 0i32;
            for cmd in commands {
                let cost = match cmd {
                    Cmd::Noop            => 0,
                    Cmd::Add(x) if x < 0 => return Err("neg add"),
                    Cmd::Add(x)          => x,
                    Cmd::Mul(x) if x == 0 => return Err("mul zero"),
                    Cmd::Mul(x)          => x * 2,
                    Cmd::Done            => break,
                };
                total += cost;
            }
            Ok(total)
        }
    };
    group.bench_function("match-5-arm-inside-for-loop", |b| {
        b.iter_batched(
            || match_in_loop.clone(),
            |blk| black_box(lower_block(blk)),
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

// ============================================================================
// G. ANF HOISTING STORM — sub-expression extraction at every position
// ============================================================================
fn bench_g_anf_hoisting(c: &mut Criterion) {
    let mut group = c.benchmark_group("G. ANF Hoisting Storm");

    // G1: Tuple with 4 hoistable elements
    let tuple_hoist: Block = parse_quote! {
        {
            let t = (
                if c1 { return 1; } else { fetch_a() },
                { let x = compute_b(); x * 2 },
                if c2 { return 2; } else { fetch_c() },
                if c3 { return 3; } else { fetch_d() },
            );
            t.0 + t.1 + t.2 + t.3
        }
    };
    group.bench_function("tuple-4-hoistable-elements", |b| {
        b.iter_batched(
            || tuple_hoist.clone(),
            |blk| black_box(lower_block(blk)),
            BatchSize::SmallInput,
        )
    });

    // G2: Array with 6 hoistable elements
    let array_hoist: Block = parse_quote! {
        {
            let arr = [
                if c1 { return 0; } else { a * 1 },
                if c2 { return 0; } else { b * 2 },
                if c3 { return 0; } else { c * 3 },
                if c4 { return 0; } else { d * 4 },
                if c5 { return 0; } else { e * 5 },
                if c6 { return 0; } else { f * 6 },
            ];
            arr[0] + arr[1] + arr[2]
        }
    };
    group.bench_function("array-6-hoistable-elements", |b| {
        b.iter_batched(
            || array_hoist.clone(),
            |blk| black_box(lower_block(blk)),
            BatchSize::SmallInput,
        )
    });

    // G3: Function call with 4 hoistable arguments
    let fn_call_hoist: Block = parse_quote! {
        {
            compute(
                if c1 { return -1; } else { a + 1 },
                { let tmp = transform(b); tmp * 2 },
                if c2 { return -2; } else { c.method() },
                d,
            )
        }
    };
    group.bench_function("fn-call-4-hoistable-args", |b| {
        b.iter_batched(
            || fn_call_hoist.clone(),
            |blk| black_box(lower_block(blk)),
            BatchSize::SmallInput,
        )
    });

    // G4: Deep arithmetic chain
    let deep_arithmetic: Block = parse_quote! {
        {
            let r = (fetch_a() + fetch_b())
                * (fetch_c() - fetch_d())
                + (fetch_e() / fetch_f())
                - (fetch_g() * fetch_h());
            r
        }
    };
    group.bench_function("deep-arithmetic-anf", |b| {
        b.iter_batched(
            || deep_arithmetic.clone(),
            |blk| black_box(lower_block(blk)),
            BatchSize::SmallInput,
        )
    });

    // G5: Struct literal with hoistable field expressions
    let struct_hoist: Block = parse_quote! {
        {
            let pt = Point {
                x: if cx { return -1; } else { ax + bx },
                y: if cy { return -2; } else { ay * by },
                z: { let tmp = az - bz; tmp.abs() },
            };
            pt
        }
    };
    group.bench_function("struct-3-hoistable-fields", |b| {
        b.iter_batched(
            || struct_hoist.clone(),
            |blk| black_box(lower_block(blk)),
            BatchSize::SmallInput,
        )
    });

    // G6: Hoistable expressions in binary LHS and RHS
    let binary_hoist: Block = parse_quote! {
        {
            let r = (if a { return 0; } else { fetch_l() })
                  + (if b { return 0; } else { fetch_r() })
                  * (if c { return 0; } else { fetch_m() });
            r
        }
    };
    group.bench_function("binary-op-both-sides-hoistable", |b| {
        b.iter_batched(
            || binary_hoist.clone(),
            |blk| black_box(lower_block(blk)),
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

// ============================================================================
// H. END-TO-END PIPELINE — lower + quote wall-clock cost per function
// ============================================================================
fn bench_h_end_to_end(c: &mut Criterion) {
    let mut group = c.benchmark_group("H. End-to-End Pipeline");

    let f_simple_guard: ItemFn = parse_quote! {
        pub fn guard(x: i32) -> i32 {
            if x < 0 { return 0; }
            x * 2
        }
    };
    group.bench_function("e2e-simple-guard", |b| {
        b.iter_batched(
            || f_simple_guard.clone(),
            |func| {
                let lowered = lower_fn(func);
                black_box(quote::quote!(#lowered))
            },
            BatchSize::SmallInput,
        )
    });

    let f_for_loop: ItemFn = parse_quote! {
        pub fn sum_evens(items: Vec<i32>, abort: i32) -> Result<i32, &'static str> {
            let mut s = 0i32;
            for x in items {
                if x == abort { return Err("aborted"); }
                if x % 2 != 0 { continue; }
                s += x;
            }
            Ok(s)
        }
    };
    group.bench_function("e2e-for-loop-signals", |b| {
        b.iter_batched(
            || f_for_loop.clone(),
            |func| {
                let lowered = lower_fn(func);
                black_box(quote::quote!(#lowered))
            },
            BatchSize::SmallInput,
        )
    });

    let f_nested_labeled: ItemFn = parse_quote! {
        pub fn find_target(grid: &[[i32; 8]; 8], target: i32) -> Option<(usize, usize)> {
            'row: for r in 0..8usize {
                for c in 0..8usize {
                    if grid[r][c] == target { return Some((r, c)); }
                    if grid[r][c] < 0 { break 'row; }
                }
            }
            None
        }
    };
    group.bench_function("e2e-nested-labeled-loops", |b| {
        b.iter_batched(
            || f_nested_labeled.clone(),
            |func| {
                let lowered = lower_fn(func);
                black_box(quote::quote!(#lowered))
            },
            BatchSize::SmallInput,
        )
    });

    let f_try_for: ItemFn = parse_quote! {
        pub fn process_all(items: Vec<i32>) -> Result<i32, String> {
            let mut total = 0i32;
            for x in items {
                let raw = fetch(x)?;
                let parsed = parse(raw)?;
                total += parsed;
            }
            Ok(total)
        }
    };
    group.bench_function("e2e-try-in-for-loop", |b| {
        b.iter_batched(
            || f_try_for.clone(),
            |func| {
                let lowered = lower_fn(func);
                black_box(quote::quote!(#lowered))
            },
            BatchSize::SmallInput,
        )
    });

    let f_complex: ItemFn = parse_quote! {
        pub fn dispatch_pipeline(
            commands: Vec<Cmd>,
            max_budget: i32,
        ) -> Result<i32, &'static str> {
            let mut budget = max_budget;
            let mut score = 0i32;
            for cmd in commands {
                let cost = match cmd {
                    Cmd::Noop              => 0,
                    Cmd::Add(x) if x < 0  => return Err("negative add"),
                    Cmd::Add(x)            => x,
                    Cmd::Mul(x) if x == 0  => return Err("mul by zero"),
                    Cmd::Mul(x)            => {
                        let v = fetch_mul(x)?;
                        score *= v;
                        0
                    }
                    Cmd::Check { a, b }    => {
                        if a > b { return Err("check failed"); }
                        a + b
                    }
                    Cmd::Loop { n, step }  => {
                        let mut acc = 0i32;
                        for _ in 0..n {
                            if budget <= 0 { break; }
                            acc += step;
                            budget -= 1;
                        }
                        acc
                    }
                };
                if budget < cost { return Err("budget exceeded"); }
                budget -= cost;
                score  += cost;
            }
            Ok(score)
        }
    };
    group.bench_function("e2e-complex-realistic", |b| {
        b.iter_batched(
            || f_complex.clone(),
            |func| {
                let lowered = lower_fn(func);
                black_box(quote::quote!(#lowered))
            },
            BatchSize::SmallInput,
        )
    });

    // H6: Batch-20 — lower 20 distinct functions per iteration
    let funcs: Vec<ItemFn> = vec![
        parse_quote! { pub fn f01(x: i32) -> i32 { if x < 0 { return 0; } x + 1 } },
        parse_quote! { pub fn f02(a: i32, b: i32) -> i32 { let r = a + b; if r > 100 { return 100; } r } },
        parse_quote! { pub fn f03(items: Vec<i32>) -> i32 { let mut s = 0i32; for x in items { s += x; } s } },
        parse_quote! {
            pub fn f04(items: Vec<i32>, abort: i32) -> Result<i32, &'static str> {
                let mut s = 0i32; for x in items { if x == abort { return Err("abort"); } s += x; } Ok(s)
            }
        },
        parse_quote! { pub fn f05(n: i32) -> i32 { let mut i = 0i32; let mut s = 0i32; while i < n { s += i; i += 1; } s } },
        parse_quote! {
            pub fn f06(a: bool, b: bool, c: bool, v: i32) -> i32 {
                let t = (if a { return 1; } else { v + 1 }, if b { return 2; } else { v + 2 }, if c { return 3; } else { v + 3 });
                t.0 + t.1 + t.2
            }
        },
        parse_quote! { pub fn f07(code: i32) -> &'static str { match code { 0 => "zero", 1 => "one", 2 => "two", _ => "many" } } },
        parse_quote! {
            pub fn f08(x: i32, y: i32) -> i32 {
                if x > 0 { if y > 0 { return x + y; } return x; }
                if y > 0 { return y; }
                0
            }
        },
        parse_quote! {
            pub fn f09(items: Vec<i32>) -> Result<i32, &'static str> {
                let mut acc = 0i32; for x in items { let v = parse_item(x)?; acc += v; } Ok(acc)
            }
        },
        parse_quote! {
            pub fn f10(threshold: i32) -> i32 {
                let r = loop { let v = next(); if v > threshold { break v; } if v < 0 { continue; } }; r
            }
        },
        parse_quote! {
            pub fn f11(a: i32, b: i32, c: i32, d: i32) -> i32 {
                if a > 0 { if b > 0 { if c > 0 { if d > 0 { return a*b*c*d; } return 0; } return 0; } return 0; }
                0
            }
        },
        parse_quote! {
            pub fn f12(items: Vec<i32>, limit: i32) -> i32 {
                let mut sum = 0i32;
                'outer: for x in items.iter() { for &y in items.iter() {
                    if x + y > limit { break 'outer; } sum += x + y;
                }} sum
            }
        },
        parse_quote! {
            pub fn f13(v: i32) -> i32 {
                match v { x if x < 0 => return -1, 0 => return 0, x if x < 10 => x * 2, x if x < 100 => x + 5, _ => return 100 }
            }
        },
        parse_quote! {
            pub fn f14(items: Vec<Option<i32>>) -> i32 {
                let mut total = 0i32;
                for opt in items { let Some(v) = opt else { continue; }; if v < 0 { break; } total += v; }
                total
            }
        },
        parse_quote! {
            pub fn f15(a: i32, b: i32) -> Result<i32, &'static str> {
                let ra = helper_a(a)?; let rb = helper_b(b)?; Ok(ra + rb)
            }
        },
        parse_quote! {
            pub fn f16(mut count: i32) -> i32 {
                while let Some(v) = next_opt() { if count == 0 { return -1; } count -= v; } count
            }
        },
        parse_quote! {
            pub fn f17(items: Vec<i32>, abort: i32, limit: i32) -> Result<i32, &'static str> {
                let mut s = 0i32;
                for x in items { if x == abort { return Err("abort"); } let v = parse(x)?; if v > limit { break; } s += v; }
                Ok(s)
            }
        },
        parse_quote! {
            pub fn f18(x: i32, flag: bool) -> i32 {
                let v = -(if flag { return 0; } else { x + 1 }); v * 2
            }
        },
        parse_quote! {
            pub fn f19(a: i32, b: i32, c: i32) -> i32 {
                let arr = [a * b, if a > 0 { return 1; } else { b + c }, c * 3]; arr[0] + arr[1] + arr[2]
            }
        },
        parse_quote! {
            pub fn f20(items: Vec<i32>) -> Result<i32, &'static str> {
                let mut total = 0i32;
                for chunk in items.chunks(4) {
                    let mut cs = 0i32;
                    for &x in chunk { if x < 0 { return Err("neg"); } let v = validate(x)?; cs += v; }
                    if cs > 1000 { break; }
                    total += cs;
                }
                Ok(total)
            }
        },
    ];

    group.throughput(Throughput::Elements(20));
    group.bench_function("e2e-batch-20-functions", |b| {
        b.iter_batched(
            || funcs.clone(),
            |batch| {
                for f in batch {
                    let lowered: ItemFn = lower_fn(f);
                    black_box(lowered);
                }
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_a_flat_sequences,
    bench_b_branching_growth,
    bench_c_loop_desugaring,
    bench_d_stack_signal_pressure,
    bench_e_try_in_loops,
    bench_f_match_arms,
    bench_g_anf_hoisting,
    bench_h_end_to_end,
);
criterion_main!(benches);
