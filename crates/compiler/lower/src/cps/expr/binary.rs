use crate::ir::scoped::{
    CpsScoped, CpsScopedBlock, CpsScopedCont, CpsScopedContinuation, CpsTerminatorScopedContUse,
};
use crate::ir::{CpsBody, CpsTerminator};
use crate::{
    Result,
    cps::continuation::ScopeEnd,
    cps::expr::CpsBind,
    cps::expr::branches::{HoistTargetTest, default_hoist_test},
    cps::{CpsContinuation, CpsCtx, CpsStep},
    derive_cps_bind,
    ir::CpsBlock,
    ir::CpsExprBinary,
    ir::scoped::CpsElse,
    util::syn::{make_false, make_syn_if_else_with_attr, make_true},
};
use syn::Ident;
use syn::token::{If, Not};
use syn::{Attribute, BinOp, Expr, ExprBinary, ExprIf, ExprUnary, UnOp};

impl CpsStep for ExprBinary {
    #[inline]
    fn then(self, c: CpsContinuation, ctx: &mut CpsCtx) -> Result<CpsContinuation> {
        match desugar_binary(self) {
            Ok(bin) => bin.then(c, ctx),
            Err(cond) => cond.then(c, ctx),
        }
    }
}
impl CpsBind for ExprBinary {
    fn and_then(
        self,
        bind: Ident,
        continuation: CpsContinuation,
        ctx: &mut CpsCtx,
    ) -> Result<CpsContinuation> {
        match desugar_binary(self) {
            Ok(bin) => bin.and_then(bind, continuation, ctx),
            Err(cond) => cond.and_then(bind, continuation, ctx),
        }
    }
}

impl HoistTargetTest for ExprBinary {
    fn with_hoisting_test(
        self,
        scope_end: ScopeEnd,
        if_token: If,
        then_branch: CpsBlock,
        else_branch: CpsElse,
        ctx: &mut CpsCtx,
    ) -> Result<CpsContinuation>
    where
        Self: Sized,
    {
        let bin = desugar_binary_(self);
        match bin {
            Ok(bin) => {
                // this sucks cos it has to be desugared again
                let bin = bin.into_binary();
                default_hoist_test(bin, scope_end, if_token, then_branch, else_branch, ctx)
            }
            Err(cond) => {
                /*
                    if cond {
                        then_block
                    } else {
                        else_block
                    }

                    where cond: if some_scrutinee { cond_then_block } else { True | False }

                    we have;

                    if some_scrutinee {
                        if cond_then_block {
                            then_block
                        } else {
                            else_block
                        }
                     } else {
                        then_block (on True) or else_block (on False)
                     }
                */
                // The then_block would seem to have access to more variables than the else_block.
                // This is true with False when else_block is duplicated as in
                /*
                    if a && let Some(var) = v {
                        println!("{var}")
                    }
                */
                // But with `a || let Some(var) = v` it's false, but the truth is, that cond is invalid
                // rust... which means we can group freely...

                // we make the duplicated one the continuation of the whole if
                // and the original cont the continuation of a fake block enclosing the 'if'

                fn make_scope_end(ctx: &mut CpsCtx, body: CpsBody) -> CpsScopedContinuation {
                    let cont_id = ctx.next_temp_ident("bin_continuation");
                    CpsScopedCont {
                        ident: cont_id,
                        binding: None,
                        body: CpsContinuation::from(body),
                        is_branch: false,
                    }
                    .into()
                }

                fn make_cont_use(c: &CpsScopedContinuation) -> CpsBody {
                    CpsBody::from(CpsTerminator::ContUse(CpsTerminatorScopedContUse {
                        scope_continuation: c.clone(),
                    }))
                }

                let (then_block, else_block, duplicated) = match cond.else_branch {
                    ElseValue::True => {
                        // we clone then_block
                        let then = make_scope_end(ctx, then_branch.body);
                        (
                            CpsBlock {
                                brace_token: then_branch.brace_token,
                                body: make_cont_use(&then),
                            },
                            else_branch,
                            then,
                        )
                    }
                    ElseValue::False => {
                        // we clone else_block
                        let else_ = make_scope_end(ctx, else_branch.block.body);

                        (
                            then_branch,
                            CpsElse {
                                else_token: else_branch.else_token,
                                block: CpsBlock {
                                    brace_token: else_branch.block.brace_token,
                                    body: make_cont_use(&else_),
                                },
                            },
                            else_,
                        )
                    }
                };

                let inner_if = (*cond.then_branch).with_hoisting_test(
                    ScopeEnd::hole(),
                    if_token,
                    then_block,
                    else_block,
                    ctx,
                )?;

                let dup_use = make_cont_use(&duplicated);
                let whole_if = (*cond.scrutinee).with_hoisting_test(
                    ScopeEnd::new_owned(ctx, duplicated),
                    if_token,
                    inner_if.into(),
                    CpsElse {
                        else_token: Default::default(),
                        block: dup_use.into(),
                    },
                    ctx,
                )?;

                let group = CpsBlock::from(whole_if);

                Ok(scope_end.into_terminator(CpsScoped::Block(CpsScopedBlock { block: group })))
            }
        }
    }
}

derive_cps_bind! {
    NonShortCircuitBinary {
        attrs,
        (pub left),
        op,
        (pub right),
    } -> {
        CpsExprBinary
    }
}

#[derive(Debug)]
struct NonShortCircuitBinary {
    pub attrs: Vec<Attribute>,
    pub left: Box<Expr>,
    pub op: BinOp,
    pub right: Box<Expr>,
}

#[derive(Debug)]
struct BooleanBinary {
    pub attrs: Vec<Attribute>,
    pub scrutinee: Box<Expr>,
    pub then_branch: Box<Expr>,
    pub else_branch: ElseValue,
}

#[derive(Debug, Copy, Clone)]
enum ElseValue {
    True,
    False,
}

impl NonShortCircuitBinary {
    fn into_binary(self) -> ExprBinary {
        ExprBinary {
            attrs: self.attrs,
            left: self.left,
            op: self.op,
            right: self.right,
        }
    }
}

impl BooleanBinary {
    fn into_if(self) -> ExprIf {
        make_syn_if_else_with_attr(
            self.attrs,
            self.scrutinee,
            *self.then_branch,
            match self.else_branch {
                ElseValue::True => make_true(),
                ElseValue::False => make_false(),
            },
        )
    }
}

// result as either
fn desugar_binary(expr_binary: ExprBinary) -> std::result::Result<NonShortCircuitBinary, ExprIf> {
    desugar_binary_(expr_binary).map_err(|b| b.into_if())
}

fn desugar_binary_(
    expr_binary: ExprBinary,
) -> std::result::Result<NonShortCircuitBinary, BooleanBinary> {
    match expr_binary.op {
        BinOp::And(_) if !(is_simple(&expr_binary.left) && is_simple(&expr_binary.right)) => {
            // only run right if left is true
            let cond = expr_binary.left;
            Err(BooleanBinary {
                attrs: expr_binary.attrs,
                scrutinee: cond,
                then_branch: expr_binary.right,
                else_branch: ElseValue::False,
            })
        }
        BinOp::Or(or) if !(is_simple(&expr_binary.left) && is_simple(&expr_binary.right)) => {
            // only run right if left is false
            let cond = Expr::Unary(ExprUnary {
                attrs: vec![],
                op: UnOp::Not(Not(or.spans[0])),
                expr: expr_binary.left,
            });

            Err(BooleanBinary {
                attrs: expr_binary.attrs,
                scrutinee: cond.into(),
                then_branch: expr_binary.right,
                else_branch: ElseValue::True,
            })
        }
        _ => Ok(NonShortCircuitBinary {
            attrs: expr_binary.attrs,
            left: expr_binary.left,
            op: expr_binary.op,
            right: expr_binary.right,
        }),
    }
}

// false doesn't mean it isn't
fn is_simple(expr: &Expr) -> bool {
    match expr {
        Expr::Lit(_) => true,
        Expr::Path(_) => true,
        Expr::Unary(e) => is_simple(&e.expr),
        Expr::Paren(expr) => is_simple(&expr.expr),
        Expr::Binary(bin) => is_simple(&bin.left) && is_simple(&bin.right),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::CpsError;

    #[test]
    #[should_panic]
    #[allow(unused_variables, unused_mut)]
    #[allow(unreachable_code)]
    fn assign_cond() {
        let mut input = true;
        input |= {
            panic!();
            true
        };
    }

    #[test]
    #[allow(clippy::diverging_sub_expression)]
    fn assign_cond_expanded() {
        let mut input = true;
        input = input || {
            unreachable!();
        };

        assert!(input);
    }

    #[test]
    fn attribute_rejection_on_binary_expr() {
        use syn::{ExprBinary, parse_quote};

        let expr: ExprBinary = parse_quote!(
            #[inline]
            a + b
        );
        let err = expr.step().unwrap_err();
        assert!(matches!(*err, CpsError::UnallowedAttributes(..)));
    }
}
