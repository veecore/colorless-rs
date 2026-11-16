use crate::Fold;
use crate::Result;
use crate::context::FoldContext;
use crate::ir::Folded;
use crate::rules::branch::util::{catch_arms, throw_arms_unfolded};
use colorless_lower::ir::IntoSyn;
use colorless_lower::ir::scoped::{CpsScopedIf, CpsScopedMatch};
use syn::{Arm, Block, Expr, ExprIf, ExprMatch, Stmt};

mod util;

impl Fold for CpsScopedIf {
    type Output = Folded;

    fn fold(self, _below: (), ctx: &mut FoldContext) -> Result<Self::Output> {
        let then_block_token = self.then_branch.brace_token;

        let thrown =
            throw_arms_unfolded(ctx, [self.then_branch, self.else_branch.block].into_iter())?;
        let mut arms = thrown.arms.into_iter();

        let then_expr = arms.next().expect("the two input arms are preserved");
        let else_expr = arms.next().expect("the two input arms are preserved");
        debug_assert!(arms.next().is_none());

        let branch = Expr::If(ExprIf {
            attrs: vec![],
            if_token: self.if_token,
            cond: self.cond.into_syn(),
            then_branch: Block {
                brace_token: then_block_token,
                stmts: vec![Stmt::Expr(then_expr, None)],
            },
            else_branch: Some((self.else_branch.else_token, else_expr.into())),
        });

        Ok(catch_arms(ctx, thrown.facts, branch))
    }
}

impl Fold for CpsScopedMatch {
    type Output = Folded;

    fn fold(self, _below: (), ctx: &mut FoldContext) -> Result<Self::Output> {
        let arm_count = self.arms.len();
        let mut folded = Vec::with_capacity(arm_count);
        let mut parts = Vec::with_capacity(arm_count);

        for arm in self.arms {
            parts.push((
                arm.pat,
                arm.fat_arrow_token,
                arm.body.brace_token,
                arm.comma,
            ));
            folded.push(arm.body);
        }

        let thrown = throw_arms_unfolded(ctx, folded.into_iter())?;
        let arms = parts
            .into_iter()
            .zip(thrown.arms)
            .map(|((pat, fat_arrow_token, brace_token, comma), expr)| Arm {
                attrs: Vec::new(),
                pat,
                fat_arrow_token,
                body: Box::new(Expr::Block(syn::ExprBlock {
                    attrs: Vec::new(),
                    label: None,
                    block: Block {
                        brace_token,
                        stmts: vec![Stmt::Expr(expr, None)],
                    },
                })),
                comma,
            })
            .collect();

        let branch = Expr::Match(ExprMatch {
            attrs: Vec::new(),
            match_token: self.match_token,
            expr: self.expr.into_syn(),
            brace_token: self.brace_token,
            arms,
        });

        Ok(catch_arms(ctx, thrown.facts, branch))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Config;
    use crate::ir::FoldedOutput;
    use colorless_lower::cps::{CpsContinuation, CpsStep};
    use colorless_lower::ir::CpsBody;
    use quote::quote;
    use syn::parse_quote;

    fn fold_match(match_: ExprMatch) -> Folded {
        let lowered: CpsContinuation = match_.step().unwrap();
        CpsBody::from(lowered)
            .fold((), &mut FoldContext::new(Config::default()))
            .unwrap()
    }

    fn folded_tokens(folded: &Folded) -> String {
        let stmts = folded.stmts.iter();
        quote!(#(#stmts)*).to_string()
    }

    #[test]
    fn empty_match_needs_no_carrier() {
        let folded = fold_match(parse_quote!(match never {}));
        let generated = folded_tokens(&folded);

        assert_eq!(folded.output, FoldedOutput::default());
        assert!(generated.starts_with("match never"), "{generated}");
        assert!(!generated.contains("OneOf"), "{generated}");
    }

    #[test]
    fn single_arm_match_does_not_invent_one_of_one() {
        let folded = fold_match(parse_quote!(match value {
            _ => 7,
        }));
        let generated = folded_tokens(&folded);

        assert_eq!(folded.output, FoldedOutput::default());
        assert!(generated.contains("match value"), "{generated}");
        assert!(!generated.contains("OneOf1 >"), "{generated}");
        assert!(!generated.contains("Ready"), "{generated}");
    }
}
