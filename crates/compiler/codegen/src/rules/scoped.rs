use crate::Fold;
use crate::Result;
use crate::context::{ContinuationFact, ContinuationFactContainer, FoldContext};
use crate::ir::Folded;
use crate::util::{macro_, stmts};
use colorless_lower::ir::CpsBody;
use colorless_lower::ir::scoped::{
    CpsScoped, CpsScopedContinuation, CpsTerminatorScoped, CpsTerminatorScopedContUse,
};
use proc_macro2::{Ident, TokenStream};
use quote::{TokenStreamExt, format_ident, quote};
use std::mem;
use std::ops::Deref;
use std::rc::Rc;
use syn::{Expr, ExprMacro, Macro, MacroDelimiter, Path, parse_quote};

impl Fold for CpsTerminatorScoped {
    type Output = Folded;

    fn fold(self, _below: (), ctx: &mut FoldContext) -> Result<Self::Output> {
        let mut continuation = None;

        if let Some(cont) = &self.scope_continuation
            && Rc::strong_count(cont) > 1
        {
            let cont = cont.deref().clone();
            let body = CpsBody::from(cont.body);
            let folded = body.fold((), ctx)?;
            let name = cont.ident;

            let body = folded.stmts;
            ctx.store_continuation_fact(
                name.clone(),
                ContinuationFact {
                    output: folded.output,
                    container: ContinuationFactContainer::Macro,
                    rehydrate_requests: Default::default(),
                },
            );

            continuation = Some((name, body, cont.binding.clone(), cont.is_branch));
        };

        // DO NOT HONOR THE REQUESTS FROM YOUR CONTINUATION
        let cont_requests_fork = mem::take(&mut ctx.shadow_request);

        let folded = match self.scoped {
            CpsScoped::Block(block) => block.fold((), ctx),
            CpsScoped::If(if_) => if_.fold((), ctx),
            CpsScoped::Loop(loop_) => loop_.fold((), ctx),
            CpsScoped::Match(match_) => match_.fold((), ctx),
            CpsScoped::Unsafe(unsafe_) => unsafe_.fold((), ctx),
        };

        let mut folded = folded?;

        if let Some((id, body, binding, is_branch)) = continuation {
            let f = ctx.remove_continuation_fact(&id).expect("my continuation");
            let state = ctx.fresh_ident("state");

            let shadow_ty = ctx.shadow_path();
            let shadow = ctx.shadow_request.iter();
            let rehydrate = f.rehydrate_requests.iter();

            let mut macro_rule_1 = TokenStream::new();
            let mut macro_rule_2 = TokenStream::new();
            let mut macro_body_1 = TokenStream::new();
            let mut macro_body_2 = TokenStream::new();
            let mut declare_unify = None;

            macro_body_1.extend(quote! {
                #(let #shadow: #shadow_ty;)*
            });

            if rehydrate.len() != 0 {
                macro_rule_2.extend(quote! {
                    $#state:ident
                });
                macro_body_2.extend(quote! {
                    (#(#rehydrate,)*) = $#state;
                });
            }

            if let Some(bind) = binding {
                macro_rule_1.extend(quote! {
                    $#bind:ident
                });

                if is_branch {
                    let type_store = format_ident!("__{bind}_ty");

                    declare_unify = Some(parse_quote! {
                        let #type_store = ::core::marker::PhantomData;
                    });

                    let unify_value = ctx.unify_value_path();
                    macro_body_1.extend(quote! {
                        let #bind = #unify_value($#bind, #type_store);
                    });
                } else {
                    macro_body_1.extend(quote! {
                        let #bind = $#bind;
                    });
                }

                if !macro_body_2.is_empty() {
                    macro_rule_2.extend(quote! {
                       $#bind:ident
                    });
                    macro_body_2.extend(quote! {
                        #id! {$#bind}
                    });
                }
            } else if !macro_body_2.is_empty() {
                macro_body_2.extend(quote! {
                    #id! {}
                })
            }

            macro_body_1.append_all(body);

            /*
            (v) => { }
            (s, v)
            */
            let m = parse_quote! {
                macro_rules! #id {
                    (#macro_rule_1) => {
                        #macro_body_1
                    };
                    (#macro_rule_2) => {
                        #macro_body_2
                    };
                }
            };

            folded.stmts.push_front(macro_(m));

            if let Some(d) = declare_unify {
                folded.stmts.push_front(d);
            }
        }

        // return for the one greater than us
        ctx.shadow_request.extend(cont_requests_fork);

        Ok(folded)
    }
}

impl Fold for CpsTerminatorScopedContUse {
    type Output = Folded;

    fn fold(self, _below: (), ctx: &mut FoldContext) -> Result<Self::Output> {
        CpsTerminatorScopedContUse_ {
            inner: self,
            loop_: None,
        }
        .fold((), ctx)
    }
}

pub struct CpsTerminatorScopedLoopContUse {
    pub inner: Option<CpsScopedContinuation>,
    pub loop_use: LoopUse,
}

impl Fold for CpsTerminatorScopedLoopContUse {
    type Output = Folded;

    fn fold(self, _below: (), ctx: &mut FoldContext) -> Result<Self::Output> {
        if let Some(inner) = self.inner {
            CpsTerminatorScopedContUse_ {
                inner: CpsTerminatorScopedContUse {
                    scope_continuation: inner,
                },
                loop_: Some(self.loop_use),
            }
            .fold((), ctx)
        } else {
            let rehydrate = self.loop_use.break_states;
            let s = self.loop_use.state_bind;
            let v = self.loop_use.value_bind;

            let s = parse_quote! {
                (#(#rehydrate,)*) = #s;
                #v
            };

            Ok(Folded {
                output: Default::default(),
                stmts: Vec::into(s),
            })
        }
    }
}

struct CpsTerminatorScopedContUse_ {
    inner: CpsTerminatorScopedContUse,
    loop_: Option<LoopUse>,
}

pub struct LoopUse {
    pub state_bind: Ident,
    pub value_bind: Ident,
    pub break_states: Vec<Ident>,
}

impl Fold for CpsTerminatorScopedContUse_ {
    type Output = Folded;

    fn fold(self, _below: (), ctx: &mut FoldContext) -> Result<Self::Output> {
        let c = &self.inner.scope_continuation.ident;
        let cached = ctx.get_continuation_fact(c).expect("cached");

        let mut arg = TokenStream::new();

        if let Some(l) = self.loop_ {
            if !l.break_states.is_empty() {
                arg.append(l.state_bind);
            }

            if self.inner.scope_continuation.binding.is_some() {
                // use this instead
                arg.append(l.value_bind)
            }

            cached.rehydrate_requests.extend(l.break_states);
        } else if let Some(bind) = self.inner.scope_continuation.binding.clone() {
            arg.append(bind);
        }

        let m = Expr::Macro(ExprMacro {
            attrs: vec![],
            mac: Macro {
                path: Path::from(c.clone()),
                bang_token: Default::default(),
                delimiter: MacroDelimiter::Brace(Default::default()),
                tokens: arg,
            },
        });

        Ok(Folded {
            output: cached.output.clone(),
            stmts: stmts(m),
        })
    }
}
