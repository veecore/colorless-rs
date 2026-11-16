//! Stress tests for loop signal routing correctness.
//!
//! These tests verify that the CPS transform produces bit-for-bit identical
//! results to straight-line imperative Rust across a large randomized input
//! space. They target the most complex part of the transform: five levels of
//! nested labeled loops where every loop in the stack can be the target of a
//! `break` or `continue` from any inner level, including carries of break
//! values across multiple frames.
//!
//! # How the oracle works
//!
//! `#[cps_fn(emit_original = "imperative_5level")]` lowers `stack_5level`
//! through CPS **and** emits the original untransformed function under the
//! name `imperative_5level`. Both functions receive identical inputs; any
//! divergence is a bug in the transform.
//!
//! # What `stack_5level` exercises
//!
//! Each innermost loop iteration derives a deterministic `score` from the
//! current loop counters and uses it to trigger one of ten cross-level jump
//! variants:
//!
//! | Condition | Jump |
//! |---|---|
//! | `score + acc > threshold` | `break 'l1 VALUE` — carry value to outermost |
//! | `score % 97 == 0` | `continue 'l1` — skip 4 inner levels |
//! | `score % 79 == 0` | `break 'l2` — exit level 2 |
//! | `score % 61 == 0` | `continue 'l2` |
//! | `score % 47 == 0` | `break 'l3` |
//! | `score % 31 == 0` | `continue 'l3` |
//! | `score % 23 == 0` | `break 'l4` |
//! | `score % 17 == 0` | `continue 'l4` |
//! | `score % 11 == 0` | `break 'l5` |
//! | `score % 7 == 0`  | `continue 'l5` |
//! | otherwise         | accumulate and increment `e` |

use dev_macros::cps_fn;

// ── Oracle pair ───────────────────────────────────────────────────────────────
//
// `emit_original` emits the untransformed body as `imperative_5level` so we
// have a known-correct reference with zero macro involvement.

#[cps_fn(emit_inert = false, emit_original = "imperative_5level")]
#[allow(clippy::manual_is_multiple_of)]
#[allow(clippy::let_and_return)]
pub fn stack_5level(n1: u64, n2: u64, n3: u64, n4: u64, n5: u64, threshold: u64) -> u64 {
    pub const INITIAL_ACC: u64 = 0x12345678_9ABCDEF0;

    let mut acc: u64 = INITIAL_ACC;
    let mut a: u64 = 0;

    let result = 'l1: loop {
        if a >= n1 {
            break 'l1 acc;
        }

        let mut b: u64 = 0;
        'l2: loop {
            if b >= n2 {
                break 'l2;
            }

            let mut c: u64 = 0;
            'l3: loop {
                if c >= n3 {
                    break 'l3;
                }

                let mut d: u64 = 0;
                'l4: loop {
                    if d >= n4 {
                        break 'l4;
                    }

                    let mut e: u64 = 0;
                    'l5: loop {
                        if e >= n5 {
                            break 'l5;
                        }

                        let score = (a.wrapping_mul(0x9E3779B97F4A7C15)
                            ^ b.wrapping_mul(0xBF58476D1CE4E5B9)
                            ^ c.wrapping_mul(0x94D049BB133111EB)
                            ^ d.wrapping_mul(0x4BFDF2BF5C419C69)
                            ^ e.wrapping_mul(0x35637DC052CD6573))
                        .wrapping_add(a ^ b ^ c ^ d ^ e);

                        // 1. Cross-frame break to 'l1 with value carry
                        if score.wrapping_add(acc) > threshold {
                            let carried_value = (acc ^ score).wrapping_mul(0x517CC1B727220A95);
                            break 'l1 carried_value;
                        }
                        // 2. Cross-frame continue to 'l1
                        if score % 97 == 0 {
                            a += 1;
                            continue 'l1;
                        }
                        // 3. Cross-frame break to 'l2
                        if score % 79 == 0 {
                            break 'l2;
                        }
                        // 4. Cross-frame continue to 'l2
                        if score % 61 == 0 {
                            b += 1;
                            continue 'l2;
                        }
                        // 5. Cross-frame break to 'l3
                        if score % 47 == 0 {
                            break 'l3;
                        }
                        // 6. Cross-frame continue to 'l3
                        if score % 31 == 0 {
                            c += 1;
                            continue 'l3;
                        }
                        // 7. Local break to 'l4
                        if score % 23 == 0 {
                            break 'l4;
                        }
                        // 8. Local continue to 'l4
                        if score % 17 == 0 {
                            d += 1;
                            continue 'l4;
                        }
                        // 9. Local break to 'l5
                        if score % 11 == 0 {
                            break 'l5;
                        }
                        // 10. Local continue to 'l5
                        if score % 7 == 0 {
                            e += 1;
                            continue 'l5;
                        }

                        // Normal accumulation
                        acc = acc.wrapping_add(score).rotate_left(3) ^ score;
                        e += 1;
                    }
                    d += 1;
                }
                c += 1;
            }
            b += 1;
        }
        a += 1;
    };

    result
}

// ── Deterministic RNG (no external deps) ─────────────────────────────────────

struct SimpleRng(u64);

impl SimpleRng {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.0
    }

    fn gen_range(&mut self, max: u64) -> u64 {
        if max == 0 { 0 } else { self.next_u64() % max }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// Deterministic edge cases covering boundary conditions: empty loops, single
/// iterations, threshold extremes (0 and u64::MAX), and small/medium sizes.
#[test]
fn edge_cases_match_imperative_reference() {
    #[rustfmt::skip]
    let cases: &[(u64, u64, u64, u64, u64, u64, &str)] = &[
        (0, 0, 0, 0, 0, 0,                        "all zero sizes"),
        (1, 0, 0, 0, 0, 0,                        "only n1 nonzero"),
        (0, 1, 0, 0, 0, 0,                        "only n2 nonzero"),
        (0, 0, 1, 0, 0, 0,                        "only n3 nonzero"),
        (0, 0, 0, 1, 0, 0,                        "only n4 nonzero"),
        (0, 0, 0, 0, 1, 0,                        "only n5 nonzero"),
        (1, 1, 1, 1, 1, u64::MAX,                 "1x1x1x1x1 threshold=MAX"),
        (1, 1, 1, 1, 1, 0,                        "1x1x1x1x1 threshold=0"),
        (5, 5, 5, 5, 5, u64::MAX,                 "5^5 threshold=MAX"),
        (10, 10, 10, 10, 10, 0x8000_0000_0000_0000, "10^5 threshold=mid"),
        (20, 20, 20, 20, 20, 0x0FFF_FFFF_FFFF_FFFF, "20^5 threshold=low"),
    ];

    for (i, &(n1, n2, n3, n4, n5, th, label)) in cases.iter().enumerate() {
        let reference = imperative_5level(n1, n2, n3, n4, n5, th);
        let transformed = stack_5level(n1, n2, n3, n4, n5, th);
        assert_eq!(
            reference, transformed,
            "case {i} ({label}): reference={reference:#018X} transformed={transformed:#018X}"
        );
    }
}

/// 100 000 randomized inputs drawn from a seeded LCG. The seed is fixed so
/// the test is deterministic across runs; changing the seed is a breaking
/// change requiring a reviewer sign-off.
///
/// Input ranges are capped at 15 per dimension to keep total runtime under
/// one second in debug mode. The threshold distribution is skewed: one in four
/// draws uses a small threshold (< 2³²) to exercise the value-carry path more.
#[test]
fn randomized_100k_inputs_match_imperative_reference() {
    let mut rng = SimpleRng(0xDEAD_BEEF_CAFE_1234);

    for i in 0..100_000_u64 {
        let n1 = rng.gen_range(15);
        let n2 = rng.gen_range(15);
        let n3 = rng.gen_range(15);
        let n4 = rng.gen_range(15);
        let n5 = rng.gen_range(15);
        let th = if rng.next_u64() % 4 == 0 {
            rng.next_u64() % 0x1_0000_0000 // small threshold → more value-carry hits
        } else {
            rng.next_u64()
        };

        let reference = imperative_5level(n1, n2, n3, n4, n5, th);
        let transformed = stack_5level(n1, n2, n3, n4, n5, th);

        if reference != transformed {
            panic!(
                "mismatch at iteration {i}: \
                 ({n1},{n2},{n3},{n4},{n5}) th={th:#018X} \
                 reference={reference:#018X} transformed={transformed:#018X}"
            );
        }
    }
}
