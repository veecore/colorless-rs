use crate::{
    cps::context::CpsLoopId,
    cps::context::loop_::depth::CpsLoopDepth,
    ir::CpsExpr,
    ir::scoped::{CpsScopedContinuation, CpsTerminatorScoped, CpsTerminatorScopedContUse},
};
use proc_macro2::Ident;
use std::cmp::Ordering;
use std::collections::BTreeSet;
use syn::Token;

#[derive(Clone, Debug)]
pub enum CpsTerminator {
    ContUse(CpsTerminatorScopedContUse),
    Scoped(CpsTerminatorScoped),
    Signal(CpsTerminatorSignal),
}

#[derive(Clone, Debug)]
pub struct CpsTerminatorSignal {
    pub from: CpsLoopId,
    pub kind: CpsSignal,
}

#[derive(Clone, Debug)]
pub enum CpsSignal {
    // ── in-loop signals ──────────────────────────────────────────────────────
    /// Emits `ControlFlow::Break(LnSignal::break_k(cont_state, val))`.
    BreakOuter {
        target: CpsLoopDepth,
        val: Option<CpsExpr>,
    },
    /// Emits `ControlFlow::Break(LnSignal::continue_k(that_state))`.
    ContinueOuter {
        target: CpsLoopDepth,
    },
    /// Emits `ControlFlow::Break(LnSignal::return_(expr))`.
    Return {
        val: Option<CpsExpr>,
    },
    /// Emits `ControlFlow::Continue(self_state)`.
    ContinueSelf,

    // ── signal receive arms ───────────────────────────────────────────────────
    //
    // Each variant corresponds to one arm of the signal-match generated around
    // the loop body.  The enum carries the bindings needed for that arm rather
    // than trying to unify them into a single value field.
    /// Emits `ControlFlow::Break((state, val))` — the loop broke with a value.
    ReceiveBreakSelf {
        continuation: Option<CpsScopedContinuation>,
        state_bind: Ident,
        value_bind: Ident,
    },
    ReceiveBreakSignal {
        signal_bind: Ident,
    },
    /// Emits `ControlFlow::Continue(state)` — a parent loop was continued.
    ReceiveContinueParent {
        state_bind: Ident,
    },
}

/// Loop-lowering hints parsed from `#[loop_state]` and related attributes.
#[derive(Clone, Debug, Default)]
pub struct CpsLoopHint {
    pub no_shadow: bool,
    pub default_break_state: bool,
    pub state: LoopStateInput,
}

/// One user-declared state binding threaded through a colored loop.
#[derive(Clone, Debug, Eq)]
pub struct CpsLoopBinding {
    pub thread: bool,
    pub referenced: Option<Token![ref]>,
    pub mutability: Option<Token![mut]>,
    pub ident: Ident,
}

#[derive(Clone, Debug, Default)]
pub struct LoopStateInput {
    // BTreeSet for uniqueness and stable iteration order.
    pub bindings: BTreeSet<CpsLoopBinding>,
}

impl PartialEq for CpsLoopBinding {
    fn eq(&self, other: &Self) -> bool {
        self.ident.eq(&other.ident)
    }
}

impl PartialOrd for CpsLoopBinding {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CpsLoopBinding {
    fn cmp(&self, other: &Self) -> Ordering {
        self.ident.cmp(&other.ident)
    }
}

mod convert_impls {
    use crate::{
        cps::expr::loops::{break_signal, continue_signal, return_signal},
        ir::CpsSignal,
        ir::IntoSyn,
        ir::scoped::CpsTerminatorScopedContUse,
        ir::terminator::{CpsTerminator, CpsTerminatorSignal},
        util::syn::{ident_syn_expr, make_unterminated_stmt_expr_syn},
        util::syn::{make_continue, make_syn_break, unit_syn_expr},
    };
    use quote::format_ident;
    use syn::{Expr, Path, Stmt, parse_quote};

    impl IntoSyn<Vec<Stmt>> for CpsTerminator {
        fn into_syn(self) -> Vec<Stmt> {
            match self {
                CpsTerminator::ContUse(cont) => cont.into_syn(),
                CpsTerminator::Signal(signal) => signal.into_syn(),
                CpsTerminator::Scoped(scoped) => scoped.into_syn(),
            }
        }
    }

    impl IntoSyn<Vec<Stmt>> for CpsTerminatorSignal {
        fn into_syn(self) -> Vec<Stmt> {
            let krate: Path = parse_quote!(::cps_signal);

            // By emitting signal as value, we'd be able to catch more unnecessarily appended
            // signals
            let expr: Expr = match self.kind {
                CpsSignal::BreakOuter { target, val } => {
                    // let signal
                    let state_storage = format_ident!("__loop_{}_storage", target.0);
                    let val = val.into_syn().unwrap_or_else(unit_syn_expr);
                    make_syn_break(Some(break_signal(
                        &krate,
                        self.from.depth(),
                        target,
                        parse_quote! {
                            (#state_storage!(@break_), #val)
                        },
                    )))
                }
                CpsSignal::ContinueOuter { target } => {
                    let state_storage = format_ident!("__loop_{}_storage", target.0);

                    make_syn_break(Some(continue_signal(
                        &krate,
                        self.from.depth(),
                        target,
                        parse_quote! {
                            (#state_storage!())
                        },
                    )))
                }
                CpsSignal::Return { val } => {
                    let val = val.into_syn().unwrap_or_else(unit_syn_expr);
                    make_syn_break(Some(return_signal(&krate, self.from.depth(), val)))
                }
                CpsSignal::ContinueSelf => make_continue(),

                // receive
                CpsSignal::ReceiveBreakSelf {
                    continuation,
                    state_bind,
                    value_bind,
                } => {
                    let storage = format_ident!("__loop_{}_storage", self.from.depth().0);
                    let mut stmts = Vec::from([make_unterminated_stmt_expr_syn(
                        parse_quote!(#storage!{@reset_break #state_bind}),
                    )]);
                    if let Some(continuation) = continuation {
                        if let Some(binding) = &continuation.binding {
                            stmts.push(parse_quote! {
                                let #binding = #value_bind;
                            });
                        }

                        let cont_use = CpsTerminator::ContUse(CpsTerminatorScopedContUse {
                            scope_continuation: continuation.clone(),
                        });
                        stmts.extend(cont_use.into_syn());
                    } else {
                        stmts.push(make_unterminated_stmt_expr_syn(ident_syn_expr(value_bind)));
                    }

                    return stmts;
                }
                CpsSignal::ReceiveContinueParent { state_bind } => {
                    let parent_storage =
                        format_ident!("__loop_{}_storage", self.from.depth().0 - 1);

                    let reset_all = parse_quote!(#parent_storage!{@reset_all #state_bind});
                    let continue_ = make_continue();

                    return [reset_all, make_unterminated_stmt_expr_syn(continue_)].into();
                }
                CpsSignal::ReceiveBreakSignal { signal_bind } => {
                    return parse_quote! {
                        break #signal_bind;
                    };
                }
            };
            let stmt = make_unterminated_stmt_expr_syn(expr);
            vec![stmt]
        }
    }
}
