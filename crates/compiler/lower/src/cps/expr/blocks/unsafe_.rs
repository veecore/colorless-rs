use crate::cps::continuation::ScopeEnd;
use crate::{
    CpsStep, Result,
    cps::{CpsBind, CpsContinuation, CpsCtx},
    derive_cps_bind_scope_delegate,
    error::assert_no_attrs,
    ir::CpsBlock,
    ir::scoped::{CpsScoped, CpsScopedUnsafe},
};
use syn::{ExprUnsafe, Ident};

derive_cps_bind_scope_delegate!(ExprUnsafe => lower_unsafe / end_non_branch_scope);

fn lower_unsafe(uns: ExprUnsafe, scope_end: ScopeEnd, ctx: &mut CpsCtx) -> Result<CpsContinuation> {
    assert_no_attrs(uns.attrs)?;

    let cont = scope_end.continuation().cloned();

    let brace_token = uns.block.brace_token;
    let lowered: CpsContinuation = uns.block.maybe_then(cont.clone(), ctx)?;

    let block = CpsBlock {
        brace_token,
        body: lowered.into(),
    };

    Ok(
        scope_end.into_terminator(CpsScoped::Unsafe(CpsScopedUnsafe {
            unsafe_token: uns.unsafe_token,
            block,
        })),
    )
}
