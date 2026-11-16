use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use dev_macros::cps_fn;
use std::hint::black_box;
use std::time::Duration;

pub const INITIAL_ACC: u64 = 0x12345678_9ABCDEF0;

#[cps_fn(emit_inert = false, emit_original = "imperative_5level")]
#[allow(clippy::manual_is_multiple_of)]
pub fn stack_5level(n1: u64, n2: u64, n3: u64, n4: u64, n5: u64, threshold: u64) -> u64 {
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

                        // 1. Erratic break 'l1 with value carry
                        if score.wrapping_add(acc) > threshold {
                            let carried_value = (acc ^ score).wrapping_mul(0x517CC1B727220A95);
                            break 'l1 carried_value;
                        }
                        // 2. Erratic continue 'l1
                        if score % 97 == 0 {
                            a += 1;
                            continue 'l1;
                        }
                        // 3. Erratic break 'l2
                        if score % 79 == 0 {
                            break 'l2;
                        }
                        // 4. Erratic continue 'l2
                        if score % 61 == 0 {
                            b += 1;
                            continue 'l2;
                        }
                        // 5. Erratic break 'l3
                        if score % 47 == 0 {
                            break 'l3;
                        }
                        // 6. Erratic continue 'l3
                        if score % 31 == 0 {
                            c += 1;
                            continue 'l3;
                        }
                        // 7. Erratic break 'l4
                        if score % 23 == 0 {
                            break 'l4;
                        }
                        // 8. Erratic continue 'l4
                        if score % 17 == 0 {
                            d += 1;
                            continue 'l4;
                        }
                        // 9. Erratic break 'l5
                        if score % 11 == 0 {
                            break 'l5;
                        }
                        // 10. Erratic continue 'l5
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

fn bench_5level_loops(c: &mut Criterion) {
    let scenarios = [
        ("Dense_10x10x10x10x10", 10, 10, 10, 10, 10, u64::MAX),
        ("Moderate_6x6x6x6x6", 6, 6, 6, 6, 6, u64::MAX),
        (
            "EarlyExit_20x20x20x20x20",
            20,
            20,
            20,
            20,
            20,
            0x0000_0000_0000_FFFF,
        ),
        ("DeepInner_2x3x4x5x100", 2, 3, 4, 5, 100, u64::MAX),
        ("BroadOuter_100x5x4x3x2", 100, 5, 4, 3, 2, u64::MAX),
        (
            "ErraticJumps_12x12x8x8x8",
            12,
            12,
            8,
            8,
            8,
            0x7FFF_FFFF_FFFF_FFFF,
        ),
    ];

    for (name, n1, n2, n3, n4, n5, th) in scenarios {
        let mut group = c.benchmark_group(name);
        group.measurement_time(Duration::from_millis(600));
        group.warm_up_time(Duration::from_millis(200));
        group.sample_size(30);

        group.bench_function(BenchmarkId::new("1_Labeled_Loop", name), |b| {
            b.iter(|| {
                imperative_5level(
                    black_box(n1),
                    black_box(n2),
                    black_box(n3),
                    black_box(n4),
                    black_box(n5),
                    black_box(th),
                )
            })
        });

        group.bench_function(BenchmarkId::new("2a_Desugared_Loop_Tokens", name), |b| {
            b.iter(|| {
                stack_5level(
                    black_box(n1),
                    black_box(n2),
                    black_box(n3),
                    black_box(n4),
                    black_box(n5),
                    black_box(th),
                )
            })
        });

        group.finish();
    }
}

criterion_group!(benches, bench_5level_loops);
criterion_main!(benches);
