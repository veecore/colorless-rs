use crate::context::FoldContext;
use crate::ir::{Folded, FoldedOutput, YieldPoint};
use crate::util::block;
use crate::{Fold, Result};
use colorless_lower::ir::CpsBlock;
use syn::{Expr, Stmt};

pub struct ThrowResult {
    pub(crate) arms: Vec<Expr>,
    pub(crate) facts: PlanInfo,
}

pub(crate) struct PlanInfo {
    colored_slots: usize,
    slot_count: usize,
}

struct PlannedArm {
    folded: Folded,
    slot: usize,
}

pub fn throw_arms_unfolded(
    ctx: &mut FoldContext,
    arms: impl ExactSizeIterator<Item = CpsBlock>,
) -> Result<ThrowResult> {
    let arms_folded = arms
        .into_iter()
        .map(|arm| arm.fold((), ctx))
        .collect::<Result<Vec<_>>>()?;

    Ok(throw_arms(ctx, arms_folded.into_iter()))
}

pub fn throw_arms(
    ctx: &mut FoldContext,
    arms: impl ExactSizeIterator<Item = Folded>,
) -> ThrowResult {
    let (planned, p) = plan_arms(ctx, arms);

    let arms = planned
        .into_iter()
        .map(|arm| route_terminal_arm(ctx, arm.folded, arm.slot, &p))
        .collect::<Vec<_>>();

    ThrowResult { arms, facts: p }
}

pub fn catch_arms(_ctx: &mut FoldContext, p: PlanInfo, branch: Expr) -> Folded {
    let catch = vec![Stmt::Expr(branch, None)];

    let branch_output = if p.colored_slots == 0 {
        FoldedOutput::default()
    } else {
        FoldedOutput::colored()
    };

    Folded {
        output: branch_output,
        stmts: catch.into(),
    }
}

fn plan_arms(
    _ctx: &mut FoldContext,
    arms: impl ExactSizeIterator<Item = Folded>,
) -> (Vec<PlannedArm>, PlanInfo) {
    let len = arms.len();
    let mut colored_slots = 0;
    let mut has_plain_slot = false;
    let mut planned = Vec::new();

    // choose arm 0 for ready so we don't deal with option later
    for (arm_idx, arm) in arms.enumerate() {
        let slot = if arm.output.requires_color() {
            colored_slots += 1;
            if arm_idx == len - 1 && !has_plain_slot {
                // take 0
                0
            } else {
                colored_slots
            }
        } else {
            has_plain_slot = true;
            0
        };

        planned.push(PlannedArm { folded: arm, slot })
    }

    let slot_count = colored_slots + usize::from(has_plain_slot);

    (
        planned,
        PlanInfo {
            colored_slots,
            slot_count,
        },
    )
}

fn route_terminal_arm(ctx: &FoldContext, arm: Folded, arm_slot: usize, p: &PlanInfo) -> Expr {
    let value = match arm.output.yield_point {
        YieldPoint::Plain if p.colored_slots != 0 => make_colored(ctx, block(arm.stmts)),
        YieldPoint::Colored | YieldPoint::Plain => block(arm.stmts),
    };

    // route
    one_of(ctx, p.slot_count, arm_slot, value)
}

fn make_colored(ctx: &FoldContext, arm: Expr) -> Expr {
    ctx.call("Ready", "ready", [arm])
}

fn one_of(ctx: &FoldContext, arm_count: usize, idx: usize, value: Expr) -> Expr {
    debug_assert!(idx < arm_count);
    if arm_count < 2 {
        value
    } else {
        route_one_of(ctx, arm_count, idx, value)
    }
}

fn route_one_of(ctx: &FoldContext, count: usize, index: usize, value: Expr) -> Expr {
    fold_one_of_route(
        ctx.config.max_one_of_arms,
        count,
        index,
        value,
        |value, width, slot| exact_one_of(ctx, width, slot, value),
    )
}

/// Fold one leaf through its carrier path from the innermost carrier to the
/// root.
///
/// The accumulator keeps production routing allocation-free while leaving the
/// arithmetic independently testable. Tests may choose a `Vec` accumulator;
/// syntax emission passes the `Expr` through directly.
fn fold_one_of_route<T>(
    max_arms: usize,
    count: usize,
    mut index: usize,
    mut value: T,
    mut wrap: impl FnMut(T, usize, usize) -> T,
) -> T {
    assert!(
        max_arms >= 2,
        "a carrier family must support at least two arms"
    );
    assert!(count > 0, "zero carrier slots need no route");
    assert!(index < count, "carrier index must name an existing slot");

    if count <= max_arms {
        if count > 1 {
            value = wrap(value, count, index);
        }
        return value;
    }

    // Step 1: O(1) largest power of B strictly below count.
    let k = (count - 1).ilog(max_arms);
    let mut target = max_arms.pow(k);

    // Step 2: Irregular leaf reduction (fold non-power-of-B leaves into target parents).
    let reduction = count - target;
    let groups = reduction.div_ceil(max_arms - 1);
    let singles = target - groups;

    if index >= singles {
        let offset = index - singles;
        let partial_width = reduction - (groups - 1) * (max_arms - 1) + 1;

        let (parent, width, slot) = if offset < partial_width {
            (singles, partial_width, offset)
        } else {
            let offset = offset - partial_width;
            let group = offset / max_arms;
            (singles + 1 + group, max_arms, offset % max_arms)
        };

        value = wrap(value, width, slot);
        index = parent;
    }

    // Step 3: All remaining levels form a perfect, uniform B-ary tree.
    while target > max_arms {
        value = wrap(value, max_arms, index % max_arms);
        index /= max_arms;
        target /= max_arms;
    }

    if target > 1 {
        value = wrap(value, target, index);
    }

    value
}

fn exact_one_of(ctx: &FoldContext, arm_count: usize, idx: usize, value: Expr) -> Expr {
    if arm_count > ctx.config.max_one_of_arms {
        unimplemented!()
    }

    let capability = format!("OneOf{arm_count}");
    let method = format!("arm_{idx}");

    ctx.call(&capability, &method, [value])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Config;
    use quote::ToTokens;
    use std::collections::HashSet;
    use syn::parse_quote;

    fn context(max_one_of_arms: usize) -> FoldContext {
        let mut config = Config::default();
        config.max_one_of_arms = max_one_of_arms;
        FoldContext::new(config)
    }

    fn tokens(value: impl ToTokens) -> String {
        value.into_token_stream().to_string()
    }

    fn one_of_route(max_arms: usize, count: usize, index: usize) -> Vec<(usize, usize)> {
        fold_one_of_route(
            max_arms,
            count,
            index,
            Vec::new(),
            |mut route, width, slot| {
                route.push((width, slot));
                route
            },
        )
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn carrier_routes_are_bounded_unique_and_uncapped() {
        for max_arms in [2, 3, 4, 12] {
            for count in [1, 2, max_arms, max_arms + 1, max_arms * max_arms, 10_000] {
                let routes = (0..count)
                    .map(|index| one_of_route(max_arms, count, index))
                    .collect::<Vec<_>>();

                assert_eq!(routes.iter().collect::<HashSet<_>>().len(), count);
                assert!(
                    routes
                        .iter()
                        .flatten()
                        .all(|&(width, slot)| { (2..=max_arms).contains(&width) && slot < width })
                );
                assert_eq!(routes[0].is_empty(), count == 1);
            }
        }
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn thirteen_slots_keep_the_first_eleven_on_the_shallow_path() {
        let routes = (0..13)
            .map(|index| one_of_route(12, 13, index))
            .collect::<Vec<_>>();

        for (index, route) in routes.iter().take(11).enumerate() {
            assert_eq!(route, &[(12, index)]);
        }
        assert_eq!(routes[11], [(2, 0), (12, 11)]);
        assert_eq!(routes[12], [(2, 1), (12, 11)]);
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn one_slot_is_a_passthrough_without_a_nonexistent_carrier() {
        let ctx = context(12);
        let generated = tokens(one_of(&ctx, 1, 0, parse_quote!(only_value)));

        assert_eq!(generated, "only_value");
        assert!(!generated.contains("OneOf1"));
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn syntax_emission_uses_only_supported_carriers() {
        let ctx = context(12);

        for count in [2, 12, 13, 145, 10_000] {
            for index in [0, count / 2, count - 1] {
                let generated = tokens(route_one_of(&ctx, count, index, parse_quote!(leaf)));

                assert!(
                    !generated.contains("OneOf1 >"),
                    "{count}:{index}: {generated}"
                );
                assert!(
                    !generated.contains("OneOf13 >"),
                    "{count}:{index}: {generated}"
                );
                assert!(generated.contains("OneOf"), "{count}:{index}: {generated}");
            }
        }
    }

    #[test]
    #[should_panic(expected = "zero carrier slots need no route")]
    fn zero_slots_are_not_routed_through_one_of() {
        let _ = one_of_route(12, 0, 0);
    }

    #[test]
    #[should_panic(expected = "carrier index must name an existing slot")]
    fn an_out_of_bounds_slot_is_rejected_before_emission() {
        let _ = one_of_route(12, 12, 12);
    }
}
