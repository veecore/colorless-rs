use crate::CpsStep;
use crate::Result;
use crate::cps::continuation::ScopeEnd;
use crate::cps::{CpsBind, CpsContinuation, CpsCtx};
use crate::derive_cps_bind_scope_delegate;
use crate::error::assert_no_attrs;
use crate::ir::CpsBlock;
use crate::ir::scoped::{CpsScoped, CpsScopedBlock};
use syn::ExprBlock;
use syn::Ident;

derive_cps_bind_scope_delegate!(ExprBlock => lower_expr_block / end_non_branch_scope);

#[inline]
fn lower_expr_block(
    block: ExprBlock,
    scope_end: ScopeEnd,
    ctx: &mut CpsCtx,
) -> Result<CpsContinuation> {
    assert_no_attrs(block.attrs)?;

    let brace_token = block.block.brace_token;

    let continuation = scope_end.continuation().cloned();

    let lowered: CpsContinuation = if let Some(label) = &block.label {
        ctx.register_block(label, continuation.clone());
        let res = block.block.maybe_then(continuation.clone(), ctx)?;
        ctx.finish_block(label);
        res
    } else {
        block.block.maybe_then(continuation.clone(), ctx)?
    };

    Ok(scope_end.into_terminator(CpsScoped::Block(CpsScopedBlock {
        block: CpsBlock {
            brace_token,
            body: lowered.into(),
        },
    })))
}
