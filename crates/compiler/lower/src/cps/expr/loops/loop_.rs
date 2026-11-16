//! CPS lowering for infinite `loop { ... }`.

use crate::{
    Result,
    cps::context::CpsLoopTarget,
    cps::continuation::ScopeEnd,
    cps::directive::take_loop_state,
    cps::expr::loops::make_break_local_pat,
    cps::expr::loops::{make_continue_parent_pat, make_signal_outer_pat, path_member},
    cps::{CpsBind, CpsContinuation, CpsCtx, CpsStep},
    derive_cps_bind_scope_delegate,
    error::assert_no_attrs,
    ir::CpsBlock,
    ir::scoped::CpsLoopMetadata,
    ir::scoped::CpsScopedMatch,
    ir::scoped::{CpsLoopSignalFacts, CpsScoped, CpsScopedLoop},
    ir::{CpsBody, CpsSignal, CpsTerminator, CpsTerminatorSignal, IntoSyn},
    util::cps::{ident_expr, make_arm, make_match_},
    util::syn::{ident_pat, ident_pat_ty_annotate},
    util::syn::{make_continue, make_stmt_expr},
};
use proc_macro2::Ident;
use quote::format_ident;
use syn::{Expr, ExprLoop, Pat, parse_quote};

derive_cps_bind_scope_delegate!(ExprLoop => lower_loop / end_non_branch_scope);

fn lower_loop(mut l: ExprLoop, scope_end: ScopeEnd, ctx: &mut CpsCtx) -> Result<CpsContinuation> {
    let (hint, attrs) = take_loop_state(l.attrs)?;
    assert_no_attrs(attrs)?;

    // Add syn manual continue instead of Cps::then(body, synthetic_continue)
    // so the logic for continue is only in the module
    l.body.stmts.push(make_stmt_expr(make_continue()));

    // Enter loop scope
    ctx.register_loop(l.label.as_ref());
    let lowered_body: CpsBlock = l.body.step_with(&mut *ctx)?;
    // Exit loop scope
    let me = ctx.finish_loop(l.label.as_ref());

    // Promote controls to direct parent
    if let Some(mut parent_loop) = ctx.current_loop() {
        me.propagate_reach_on(parent_loop.inner());
    }

    // Bind the return signal
    let signal_binding = me.signal_binding();
    let signal_alias = signal_alias(ctx, &me, ctx.signal_path());

    let cont = scope_end.continuation().cloned();
    Ok(scope_end.into_terminator(CpsScoped::Loop(CpsScopedLoop {
        cps_loop_metadata: CpsLoopMetadata {
            hint,
            scope_continuation: if me.facts_ref().can_break_self() {
                cont
            } else {
                None
            },
            facts: me.facts(),
            signal: CpsLoopSignalFacts {
                binding: signal_binding,
                signal_alias,
            },
        },
        loop_token: l.loop_token,
        body: lowered_body,
    })))
}

#[inline]
fn signal_alias(_ctx: &CpsCtx, lp: &CpsLoopTarget, runtime: &syn::Path) -> syn::Path {
    let mut specify_depth = false;
    if lp.is_outermost() {
        specify_depth = true;
    }
    let signal_outer = if !lp.can_reach_outer_scope() {
        specify_depth = true;
        "NoOuter"
    } else {
        ""
    };

    let break_self = if !lp.can_break_self() { "NoBreak" } else { "" };

    let continue_parent = if !lp.can_continue_parent() && !lp.is_outermost() {
        "NoContinueParent"
    } else {
        ""
    };

    let depth = if specify_depth {
        format_args!("{}", lp.depth())
    } else {
        format_args!("n")
    };

    path_member(
        runtime,
        &format!(
            "L{}Signal{}{}{}",
            depth, signal_outer, break_self, continue_parent
        ),
    )
}

pub fn match_signal_canonical(current_loop: &CpsLoopMetadata) -> (Pat, CpsScopedMatch) {
    let lp = &current_loop.facts;
    let signal = &current_loop.signal;

    let mut arms = Vec::with_capacity(3);

    if lp.can_break_self() {
        let state = format_ident!("__break_local_state");
        let break_val = format_ident!("__break_local_val");

        let pat = make_break_local_pat(&signal.signal_alias, parse_quote!((#state, #break_val)));

        let arm_body = CpsBody {
            stmts: Vec::new(),
            terminator: Some(
                CpsTerminator::Signal(CpsTerminatorSignal {
                    from: lp.loop_id(),
                    kind: CpsSignal::ReceiveBreakSelf {
                        continuation: current_loop.scope_continuation.clone(),
                        state_bind: state,
                        value_bind: break_val,
                    },
                })
                .into(),
            ),
        };

        let bl_arm = make_arm(pat, arm_body);
        arms.push(bl_arm);
    };

    if lp.can_reach_outer_scope() {
        let inner = format_ident!("__outer");
        let pat = make_signal_outer_pat(&signal.signal_alias, inner.clone());

        let arm_body = if lp.depth().is_outermost() {
            CpsBody::from(ident_expr(inner))
        } else {
            CpsBody {
                stmts: Vec::new(),
                terminator: Some(
                    CpsTerminator::Signal(CpsTerminatorSignal {
                        from: lp.loop_id(),
                        kind: CpsSignal::ReceiveBreakSignal { signal_bind: inner },
                    })
                    .into(),
                ),
            }
        };
        arms.push(make_arm(pat, arm_body));
    };

    if lp.can_continue_parent() && !lp.depth().is_outermost() {
        let state = format_ident!("state");

        let pat = make_continue_parent_pat(&signal.signal_alias, state.clone());

        let body = CpsBody {
            stmts: Vec::new(),
            terminator: Some(
                CpsTerminator::Signal(CpsTerminatorSignal {
                    from: lp.loop_id(),
                    kind: CpsSignal::ReceiveContinueParent {
                        state_bind: state.clone(),
                    },
                })
                .into(),
            ),
        };
        arms.push(make_arm(pat, body));
    }

    (
        signal_pat(current_loop, arms.is_empty()),
        make_match_(Box::new(ident_expr(signal.binding.clone())), arms),
    )
}

pub(crate) fn match_signal(current_loop: &CpsLoopMetadata) -> (Pat, Expr) {
    let (s, b) = match_signal_canonical(current_loop);
    (s, Expr::Match(b.into_syn()))
}

fn signal_pat(cps_loop_metadata: &CpsLoopMetadata, needs_annotation: bool) -> Pat {
    if needs_annotation {
        ident_pat_ty_annotate(
            cps_loop_metadata.signal.binding.clone(),
            cps_loop_metadata.signal.signal_alias.clone(),
        )
    } else {
        ident_pat(cps_loop_metadata.signal.binding.clone())
    }
}
