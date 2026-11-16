use crate::cps::CpsContinuation;
use crate::cps::context::CpsLoopFacts;
use crate::ir::{CpsBlock, CpsExpr, CpsLoopHint, CpsScopeId};
use proc_macro2::Ident;
use std::rc::Rc;
use syn::{Pat, Path, Token, token};

#[derive(Clone, Debug)]
pub struct CpsTerminatorScoped {
    pub scope_id: Option<CpsScopeId>,
    pub scoped: CpsScoped,
    // The Rc is good for knowing if declaring is worth it... for example
    // if c {return} else {return}; #cont
    pub scope_continuation: Option<CpsScopedContinuation>,
}

#[derive(Clone, Debug)]
pub enum CpsScoped {
    Block(CpsScopedBlock),
    If(CpsScopedIf),
    Loop(CpsScopedLoop),
    Match(CpsScopedMatch),
    Unsafe(CpsScopedUnsafe),
}

#[derive(Clone, Debug)]
pub struct CpsScopedBlock {
    pub block: CpsBlock,
}

#[derive(Clone, Debug)]
pub struct CpsScopedIf {
    pub if_token: Token![if],
    pub cond: Box<CpsExpr>,
    pub then_branch: CpsBlock,
    pub else_branch: CpsElse,
}

#[derive(Clone, Debug)]
pub struct CpsScopedLoop {
    pub cps_loop_metadata: CpsLoopMetadata,
    pub loop_token: Token![loop],
    pub body: CpsBlock,
}

#[derive(Clone, Debug)]
pub struct CpsScopedMatch {
    pub match_token: Token![match],
    pub expr: Box<CpsExpr>,
    pub brace_token: token::Brace,
    pub arms: Vec<CpsArm>,
}

#[derive(Clone, Debug)]
pub struct CpsScopedUnsafe {
    pub unsafe_token: Token![unsafe],
    pub block: CpsBlock,
}

/// The mandatory block-form `else` branch of a lowered conditional.
#[derive(Clone, Debug)]
pub struct CpsElse {
    pub else_token: Token![else],
    pub block: CpsBlock,
}

#[derive(Clone, Debug)]
pub struct CpsLoopMetadata {
    pub hint: CpsLoopHint,
    pub facts: CpsLoopFacts,
    // We can remove this and rely on the ordinary facts
    pub signal: CpsLoopSignalFacts,
    pub scope_continuation: Option<CpsScopedContinuation>,
}

#[derive(Clone, Debug)]
pub struct CpsLoopSignalFacts {
    // This is ok to remove if it signals outer (check with
    // effects)
    pub binding: Ident,
    pub signal_alias: Path,
}

#[derive(Clone, Debug)]
pub struct CpsArm {
    pub pat: Pat,
    pub fat_arrow_token: Token![=>],
    pub body: CpsBlock,
    pub comma: Option<Token![,]>,
}

pub type CpsScopedContinuation = Rc<CpsScopedCont>;

// This could be emitted as closure that takes env and output, macro or
// literally duplicated. The ContinuationUsage struct should make this
// possible
#[derive(Clone, Debug)]
pub struct CpsScopedCont {
    pub ident: Ident,
    pub body: CpsContinuation,
    // How it accesses the value of the scope
    // this is already inside body
    pub binding: Option<Ident>,
    // helps address the type annotation need with branch output
    pub is_branch: bool,
}

// Use should be Weak
// can be emitted as ident!(binding)
#[derive(Clone, Debug)]
pub struct CpsTerminatorScopedContUse {
    pub scope_continuation: CpsScopedContinuation,
}

mod convert_impls {
    use crate::cps::expr::loops::loop_::match_signal;
    use crate::util::syn::{ident_syn_expr, make_syn_block_stmts};
    use crate::{
        derive_into_syn,
        ir::scoped::{
            CpsArm, CpsElse, CpsScoped, CpsScopedBlock, CpsScopedIf, CpsScopedLoop, CpsScopedMatch,
            CpsScopedUnsafe, CpsTerminatorScoped, CpsTerminatorScopedContUse,
        },
        ir::{CpsBlock, IntoSyn},
        util::syn::{make_bind_syn, make_syn_expr_block, make_unterminated_stmt_expr_syn},
    };
    use quote::format_ident;
    use std::rc::Rc;
    use syn::punctuated::Punctuated;
    use syn::token::{Comma, Else};
    use syn::{
        Arm, Expr, ExprBlock, ExprIf, ExprLoop, ExprMatch, ExprTuple, ExprUnsafe, Stmt, parse_quote,
    };

    impl IntoSyn<Vec<Stmt>> for CpsTerminatorScoped {
        fn into_syn(self) -> Vec<Stmt> {
            let mut out = Vec::new();

            // This only helps avoid name shadowing
            if let Some(cont) = self.scope_continuation {
                let cont = Rc::unwrap_or_clone(cont);
                let name = cont.ident.clone();
                let body = CpsBlock::from(cont.body).into_syn();
                if let Some(bind) = cont.binding {
                    if cont.is_branch {
                        let type_store = format_ident!("__{bind}_ty");
                        out.push(parse_quote! {
                            let #type_store = ::core::marker::PhantomData;
                        });
                        out.push(parse_quote! {
                            macro_rules! #name {
                                ($#bind:expr) => {{
                                    // the test has to make this function available
                                    let #bind = unify_value($#bind, #type_store);
                                    #body
                                }};
                            }
                        });
                    } else {
                        out.push(parse_quote! {
                            macro_rules! #name {
                                ($#bind:expr) => {{
                                    let #bind = $#bind;
                                    #body
                                }};
                            }
                        });
                    }
                } else {
                    out.push(parse_quote! {
                        macro_rules! #name {
                            () => {{
                                #body
                            }};
                        }
                    });
                };
            }

            let mut s = self.scoped.into_syn();
            out.append(&mut s);
            out
        }
    }

    impl IntoSyn<Vec<Stmt>> for CpsTerminatorScopedContUse {
        fn into_syn(self) -> Vec<Stmt> {
            let c = self.scope_continuation.ident.clone();
            if let Some(bind) = self.scope_continuation.binding.clone() {
                parse_quote! {
                    #c!{#bind}
                }
            } else {
                parse_quote! {
                    #c!{}
                }
            }
        }
    }

    impl IntoSyn<Vec<Stmt>> for CpsScoped {
        fn into_syn(self) -> Vec<Stmt> {
            let e = match self {
                CpsScoped::Block(b) => Expr::Block(b.into_syn()),
                CpsScoped::If(i) => Expr::If(i.into_syn()),
                CpsScoped::Loop(l) => {
                    return l.into_syn();
                }
                CpsScoped::Match(m) => Expr::Match(m.into_syn()),
                CpsScoped::Unsafe(u) => Expr::Unsafe(u.into_syn()),
            };

            vec![make_unterminated_stmt_expr_syn(e)]
        }
    }

    impl IntoSyn<ExprBlock> for CpsScopedBlock {
        #[inline]
        fn into_syn(self) -> ExprBlock {
            ExprBlock {
                attrs: Vec::new(),
                label: None,
                block: self.block.into_syn(),
            }
        }
    }

    derive_into_syn! {
        ExprIf {
            _ attrs,
            if_token,
            (pub cond),
            (pub then_branch),
            (pub else_branch),
        } -> {
            CpsScopedIf
        }
    }

    impl IntoSyn<Vec<Stmt>> for CpsScopedLoop {
        fn into_syn(self) -> Vec<Stmt> {
            let (pat, match_signal) = match_signal(&self.cps_loop_metadata);

            /*
            loop {
                {break break_0()};
            }
            */
            let loop_expr = Expr::Loop(ExprLoop {
                attrs: Default::default(),
                label: Default::default(),
                loop_token: self.loop_token,
                body: self.body.into_syn(),
            });

            /*
            let signal = loop {break break_0()};
            */

            let receive_signal = make_bind_syn(pat, loop_expr);

            /*
            match signal {
                ::ContinueParent(()) => continue,
            }
            */
            let match_signal = make_unterminated_stmt_expr_syn(match_signal);

            /*
             macro_rules __loop_n_storage! {
                () => {
                    (...)
                }
             }
            */

            let loop_expr = make_syn_block_stmts([receive_signal, match_signal]);

            let state_storage =
                format_ident!("__loop_{}_storage", self.cps_loop_metadata.facts.depth().0);
            let state = self
                .cps_loop_metadata
                .hint
                .state
                .bindings
                .iter()
                .map(|i| ident_syn_expr(i.ident.clone()));

            let break_state = self
                .cps_loop_metadata
                .hint
                .state
                .bindings
                .iter()
                .filter_map(|i| {
                    if i.thread {
                        Some(ident_syn_expr(i.ident.clone()))
                    } else {
                        None
                    }
                });

            let mut total_states = state.clone().collect::<Punctuated<_, Comma>>();
            let mut total_break_states = break_state.clone().collect::<Punctuated<_, Comma>>();

            // this means if more than one of the continued loops inherit states, it'd
            // cause double move, but I'm not ready to make IntoSyn take some context...
            // codegen can handle this
            total_states.extend(self.cps_loop_metadata.facts.reached_loops().map(|d| {
                let mac = format_ident!("__loop_{}_storage", d.depth().0);
                let s: Expr = parse_quote! {
                    #mac!()
                };
                s
            }));
            total_break_states.extend(self.cps_loop_metadata.facts.reached_loops().map(|d| {
                let mac = format_ident!("__loop_{}_storage", d.depth().0);
                let s: Expr = parse_quote! {
                    #mac!(@break_)
                };
                s
            }));

            let total_states = Expr::Tuple(ExprTuple {
                attrs: vec![],
                paren_token: Default::default(),
                elems: total_states,
            });

            let total_break_states = Expr::Tuple(ExprTuple {
                attrs: vec![],
                paren_token: Default::default(),
                elems: total_break_states,
            });

            let mut total_states_reset = state.collect::<Punctuated<_, Comma>>();
            let mut total_break_states_reset = break_state.collect::<Punctuated<_, Comma>>();

            let (placeholder, pass_on_reset): (Vec<_>, Vec<_>) = self
                .cps_loop_metadata
                .facts
                .reached_loops()
                .map(|d| {
                    let placeholder = format_ident!("__loop_{}_storage_state", d.depth().0);
                    let mac = format_ident!("__loop_{}_storage", d.depth().0);
                    let s: Stmt = parse_quote! {
                        #mac!(@reset_all #placeholder);
                    };

                    total_states_reset.push(ident_syn_expr(placeholder.clone()));
                    (placeholder, s)
                })
                .unzip();

            let (break_placeholder, break_pass_on_reset): (Vec<_>, Vec<_>) = self
                .cps_loop_metadata
                .facts
                .reached_loops()
                .map(|d| {
                    let placeholder = format_ident!("__loop_{}_storage_state", d.depth().0);
                    let mac = format_ident!("__loop_{}_storage", d.depth().0);
                    let s: Stmt = parse_quote! {
                        #mac!(@reset_break #placeholder);
                    };

                    total_break_states_reset.push(ident_syn_expr(placeholder.clone()));
                    (placeholder, s)
                })
                .unzip();

            let state_reset_pat = Expr::Tuple(ExprTuple {
                attrs: vec![],
                paren_token: Default::default(),
                elems: total_states_reset,
            });

            let break_state_reset_pat = Expr::Tuple(ExprTuple {
                attrs: vec![],
                paren_token: Default::default(),
                elems: total_break_states_reset,
            });

            // TODO: REDECLARE THE STATES AS MUT
            let loop_state: Stmt = parse_quote! {
                macro_rules! #state_storage {
                    () => {{
                        #total_states
                    }};
                    (@break_) => {{
                        #total_break_states
                    }};
                    (@reset_all $state:expr) => {
                        let state = $state;
                        #(let #placeholder;)*
                        #state_reset_pat = state;
                        #(#pass_on_reset)*
                    };
                    (@reset_break $state:expr) => {
                        let state = $state;
                        #(let #break_placeholder;)*
                        #break_state_reset_pat = state;
                        #(#break_pass_on_reset)*
                    };
                }
            };

            vec![loop_state, make_unterminated_stmt_expr_syn(loop_expr)]
        }
    }

    derive_into_syn! {
         ExprMatch {
            _ attrs,
            match_token,
            (pub expr),
            brace_token,
            (pub arms),
        } -> {
            CpsScopedMatch
        }
    }

    derive_into_syn! {
        ExprUnsafe {
            _ attrs,
            unsafe_token,
            (pub block),
        } -> {
            CpsScopedUnsafe
        }
    }

    // CPS branches are always blocks, including the implicit
    // `else` continuation produced while lowering an if without source `else`.
    impl IntoSyn<Option<(Else, Box<Expr>)>> for CpsElse {
        #[inline]
        fn into_syn(self) -> Option<(Else, Box<Expr>)> {
            Some((
                self.else_token,
                Box::new(Expr::Block(ExprBlock {
                    attrs: Vec::new(),
                    label: None,
                    block: self.block.into_syn(),
                })),
            ))
        }
    }

    impl IntoSyn<Arm> for CpsArm {
        #[inline]
        fn into_syn(self) -> Arm {
            let block = self.body.into_syn();
            let body = make_syn_expr_block(block);
            Arm {
                attrs: Vec::new(),
                pat: self.pat,
                fat_arrow_token: self.fat_arrow_token,
                body: Box::new(body),
                comma: self.comma,
            }
        }
    }
}
