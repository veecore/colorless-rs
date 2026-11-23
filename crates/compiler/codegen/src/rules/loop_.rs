use crate::{
    Fold, Result,
    context::FoldContext,
    context::LoopFact,
    ir::Folded,
    rules::cps::fold_colored,
    rules::scoped::{CpsTerminatorScopedLoopContUse, LoopUse},
    runtime_call::call_once_move,
    util::block,
    util::{bind, ident_expr, ident_pat_with_mut},
    util::{bind_pat, stmts, unit},
    util::{macro_, tuple},
    util::{prepend_vecdeque, tuple_pat},
};
use colorless_lower::cps::context::{CpsLoopDepth, CpsLoopId};
use colorless_lower::cps::expr::loops::loop_::match_signal_canonical;
use colorless_lower::cps::expr::loops::{break_signal, continue_signal, return_signal};
use colorless_lower::ir::scoped::{CpsLoopMetadata, CpsScopedLoop};
use colorless_lower::ir::{CpsLoopBinding, CpsSignal, CpsTerminatorSignal, IntoSyn};
use proc_macro2::Ident;
use quote::format_ident;
use std::collections::BTreeSet;
use std::fmt::Display;
use std::mem;
use syn::token::Loop;
use syn::{Expr, ItemMacro, Pat, Stmt, parse_quote};

impl Fold for CpsScopedLoop {
    type Output = Folded;

    fn fold(self, _below: (), ctx: &mut FoldContext) -> Result<Self::Output> {
        // collect state
        let mut state_acquire =
            try_acquire_full_state(&self.cps_loop_metadata, self.loop_token, ctx)?;
        let mut state_container =
            define_state_container(&self.cps_loop_metadata, &state_acquire.full_state);

        // fold loop body
        ctx.store_loop_fact(
            self.cps_loop_metadata.facts.loop_id(),
            LoopFact {
                bindings: state_acquire.full_state,
                default_break_state: state_acquire.default_break_state,
                state_container,
            },
        );
        let mut body = self.body.fold((), ctx)?;
        let (signal_pat, signal_match) = match_signal_canonical(&self.cps_loop_metadata);
        let mut signal_match = signal_match.fold((), ctx)?;
        let my_fact = ctx
            .remove_loop_fact(self.cps_loop_metadata.facts.loop_id())
            .expect("my emotional support fact! 😭");

        // reset
        state_acquire.full_state = my_fact.bindings;
        state_container = my_fact.state_container;

        // redeclare needed macros
        body.stmts = prepend_vecdeque(
            state_acquire.reached_loops_containers_redeclare.into_iter(),
            body.stmts,
        );
        body.stmts.push_front(macro_(state_container.clone()));

        // coloring
        let mut switched_to_identity = None;
        let loop_body = if body.output.requires_color() {
            body.stmts
        } else {
            // we already injected control-flow so we can't become native loop
            let identity_color = ctx.identity_color();
            switched_to_identity = Some(mem::replace(&mut ctx.config.color_type, identity_color));

            stmts(ctx.call("Ready", "ready", [block(body.stmts)]))
        };

        // Assert non-copy body
        let state_bind = ctx.fresh_ident("state_bind");
        let closure_bind = ctx.fresh_ident("closure_bind");

        let mut prologue = state_acquire.state_rehydrate_stmts;
        prologue.extend([
            // let state_bind = (states,);
            bind(
                state_bind.clone(),
                gather_full_state_as_arg(
                    &state_acquire.full_state,
                    state_acquire.default_break_state,
                ),
            ),
            // let closure_bind = infer_loop_step(|(states,)| {..});
            bind(
                closure_bind.clone(),
                ctx.call_infer_loop_step(
                    &state_bind,
                    call_once_move(receive_state(&state_acquire.full_state), loop_body),
                ),
            ),
            // assert_non_capturing(&state_bind, closure_bind) // if really fn, it'd copy so no ref needed
            ctx.call_assert_non_capturing(&state_bind, closure_bind.clone()),
        ]);

        // C::fold(state_bind, closure_bind)
        let loop_ = ctx.call(
            "Loop",
            "fold",
            [ident_expr(state_bind), ident_expr(closure_bind)],
        );

        let mut loop_ = match switched_to_identity {
            None => fold_colored(ctx, loop_, signal_pat, signal_match),
            Some(real_color) => {
                signal_match
                    .stmts
                    .push_front(bind_pat(signal_pat, ctx.execute_identity(loop_)));
                // back to business
                ctx.config.color_type = real_color;
                signal_match
            }
        };

        loop_.stmts = prepend_vecdeque(prologue.into_iter(), loop_.stmts);

        shadow_unthreaded_states(&state_acquire.full_state, ctx);
        Ok(loop_)
    }
}

impl Fold for CpsTerminatorSignal {
    type Output = Folded;

    fn fold(self, _below: (), ctx: &mut FoldContext) -> Result<Self::Output> {
        let e = match self.kind {
            CpsSignal::BreakOuter { target, val } => ctx.break_loop(break_signal(
                ctx.cps_signal_crate(),
                self.from.depth(),
                target,
                tuple([break_state(target), val.into_syn().unwrap_or_else(unit)]),
            )),
            CpsSignal::ContinueOuter { target } => ctx.break_loop(continue_signal(
                ctx.cps_signal_crate(),
                self.from.depth(),
                target,
                continue_state(target),
            )),
            CpsSignal::Return { val } => ctx.break_loop(return_signal(
                ctx.cps_signal_crate(),
                self.from.depth(),
                val.into_syn().unwrap_or_else(unit),
            )),
            CpsSignal::ContinueSelf => ctx.continue_loop(continue_state(self.from.depth())),

            // receive
            CpsSignal::ReceiveBreakSelf {
                continuation,
                state_bind,
                value_bind,
            } => {
                // this only works in identity and for copy
                // the mem::forget trick failed
                // if we have cont redeclare instead, single cont would work but
                // pre-captured instances in other namespaces will fail...
                return CpsTerminatorScopedLoopContUse {
                    inner: continuation,
                    loop_use: LoopUse {
                        state_bind,
                        value_bind,
                        break_states: get_break_state(ctx, self.from),
                    },
                }
                .fold((), ctx);
            }
            CpsSignal::ReceiveContinueParent { state_bind } => {
                ctx.continue_loop(ident_expr(state_bind))
            }
            CpsSignal::ReceiveBreakSignal { signal_bind } => {
                ctx.break_loop(ident_expr(signal_bind))
            }
        };

        Ok(Folded {
            output: Default::default(),
            stmts: stmts(e),
        })
    }
}

fn continue_state(cps_loop_depth: CpsLoopDepth) -> Expr {
    let storage = state_storage(cps_loop_depth);
    parse_quote! {
        #storage!{}
    }
}

fn break_state(cps_loop_depth: CpsLoopDepth) -> Expr {
    let storage = state_storage(cps_loop_depth);
    parse_quote! {
        #storage!{@break_}
    }
}

struct LoopStateAcquire {
    full_state: BTreeSet<CpsLoopBinding>,
    default_break_state: bool,

    state_rehydrate_stmts: Vec<Stmt>,
    reached_loops_containers_redeclare: Vec<Stmt>,
}

fn try_acquire_full_state(
    m: &CpsLoopMetadata,
    loop_token: Loop,
    ctx: &mut FoldContext,
) -> Result<LoopStateAcquire> {
    if let Some(binding) = m
        .hint
        .state
        .bindings
        .iter()
        .find_map(|binding| binding.referenced)
    {
        // that dude introduced local borrow issues instead of solving threading
        return Err(syn::Error::new(
            binding.span,
            "This feature is not yet fully implemented. \
             Maybe convert the loop to a plain loop and manually thread the variable through break, or provide `#[default_state]` if the Default impls aren't expensive",
        ));
    }

    let mut full_state = BTreeSet::new();
    full_state.append(&mut m.hint.state.bindings.clone());
    let mut default_break_state = m.hint.default_break_state;

    let mut state_rehydrate_stmts = Vec::new();
    let mut inherits_state = false;
    let mut reached_loops_containers_redeclare = Vec::new();

    for reached_loop in m.facts.reached_loops() {
        let Some(loop_fact) = ctx.get_loop_fact(reached_loop) else {
            panic!("Expected a loop fact");
        };

        inherits_state |= !loop_fact.bindings.is_empty();
        default_break_state |= loop_fact.default_break_state;

        let storage = state_storage(reached_loop.depth());
        let st = loop_fact.bindings.iter().map(|b| &b.ident);

        // We do this because several states may overlap... this comes with the limitation
        // that one couldn't have a 'let env = something' when we emit (env,) = storage!();
        // we would have to rename
        state_rehydrate_stmts.push(parse_quote! {
            #storage! {@reacquire_all #(#st)* };
        });

        loop_fact.bindings.iter().try_for_each(|b| {
            if let Some(r) = m.hint.state.bindings.get(b) {
                // todo: reword
                return Err(syn::Error::new_spanned(&r.ident, "\
                        This loop already inherited a state with this name from an outer loop due to continuing to that loop.\n
                         If this is a new state with the same name, it is recommended to rename. This is a limitation in
                         the current rule. Note that you do not have to restate an automatically inherited state.
                         \
                        "))
            }
            full_state.insert(b.clone());
            Ok::<(), syn::Error>(())
        })?;

        reached_loops_containers_redeclare.push(macro_(loop_fact.state_container.clone()));
    }

    // we could correctly get the state that the upper loop actually needs for continuing
    // but if we do while the user expects 'env' in this inner loop, and we do provide the
    // outer 'env', it'd be wrong.... We could just decide not to provide the env under
    // the original name but that'd mean this loop would be unable to access the outer state
    // because of double move or our check... I don't think that's fair...
    // currently, we provide the recent one because we attempt to catch shadowed vars in case
    // they're not copy... when Copy, this tricks just fails...
    // I kinda believe this whole issue isn't an ast duplication issue seeing that lower's raw code
    // works just fine... it's because of fold... actually, everything is messed... if we fail at drop
    // timing and require pinky-promising, we might as well take the shamelessness all the way
    match (inherits_state, m.hint.no_shadow) {
        (true, false) => {
            let states = format_state(&full_state);
            return Err(syn::Error::new_spanned(
                loop_token,
                format!(
                    "To avoid unintentional state overwrite with Copy types, ensure that none of the inherited states {states} is redeclared then add `#[no_shadow]` on this loop. \
                        This is not an issue with your code but a serious limitation."
                ),
            ));
        }
        (false, true) => {
            return Err(syn::Error::new_spanned(
                loop_token,
                "Redundant hint `#[no_shadow]`. This is only needed when this loop inherits states",
            ));
        }
        _ => {}
    }

    Ok(LoopStateAcquire {
        full_state,
        default_break_state,
        state_rehydrate_stmts,
        reached_loops_containers_redeclare,
    })
}

fn define_state_container(m: &CpsLoopMetadata, s: &BTreeSet<CpsLoopBinding>) -> ItemMacro {
    let my_storage = state_storage(m.facts.loop_id().depth());
    let st = s.iter().map(|b| b.ident.clone()).collect::<Vec<_>>();
    let break_state = collect_break_state(s);

    parse_quote! {
        macro_rules! #my_storage {
            () => {
                (#(#st,)*)
            };
            (@break_) => {{
                (#(#break_state,)*)
            }};
            (@reacquire_all #($#st:ident)*) => {
                (#(#st,)*) = (#($#st,)*);
            };
        }
    }
}

fn get_break_state(ctx: &mut FoldContext, l: CpsLoopId) -> Vec<Ident> {
    let states = ctx.get_loop_fact(l).expect("loop fact");
    collect_break_state(&states.bindings)
}

fn collect_break_state(s: &BTreeSet<CpsLoopBinding>) -> Vec<Ident> {
    s.iter()
        .filter_map(|i| {
            if i.thread {
                Some(i.ident.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
}

// we need to specify for individual binding to avoid conflict with other loops
fn gather_full_state_as_arg(s: &BTreeSet<CpsLoopBinding>, default_break_state: bool) -> Expr {
    tuple(s.iter().map(|s| {
        if s.thread && default_break_state {
            take_break_state(&s.ident)
        } else {
            ident_expr(s.ident.clone())
        }
    }))
}

fn take_break_state(s: &Ident) -> Expr {
    let take = format_ident!("{s}_take");
    parse_quote! {{
        let #take = #s;
        #s = ::core::default::Default::default();
        #take
    }}
}

// this is to prevent using stale copy types
fn shadow_unthreaded_states(s: &BTreeSet<CpsLoopBinding>, ctx: &mut FoldContext) {
    s.iter().for_each(|s| {
        if !s.thread {
            ctx.shadow_request.insert(s.ident.clone());
        }
    });
}

fn state_storage(cps_loop_depth: CpsLoopDepth) -> Ident {
    format_ident!("__loop_{}_storage", cps_loop_depth.0)
}

fn receive_state(full_state: &BTreeSet<CpsLoopBinding>) -> Pat {
    tuple_pat(
        full_state
            .iter()
            .map(|b| ident_pat_with_mut(b.mutability.is_some(), b.ident.clone())),
    )
}

fn format_state(cps_loop_state_hint: &BTreeSet<CpsLoopBinding>) -> impl Display {
    struct D<'a>(&'a BTreeSet<CpsLoopBinding>);

    impl<'a> Display for D<'a> {
        fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let mut l = fmt.debug_list();
            for b in self.0.iter() {
                l.entry(&format_args!("`{}`", b.ident));
            }
            l.finish()
        }
    }

    D(cps_loop_state_hint)
}
