//! CPS transform for [`ExprMacro`].
//!
//! Macro invocations are treated as opaque expressions: we cannot inspect their
//! body, so we assume they produce a value without diverging.  This is
//! conservative — macro invocations that diverge (e.g. `panic!`, `return`-like
//! macros) will still compile correctly because the surrounding code is
//! unreachable and the compiler will simply warn about dead code.
//!
//! * `then`: emit `mac!(...);` as a statement and continue.
//! * `and_then`: emit `let bind = mac!(...);` and continue.

use crate::cps::CpsCtx;
use crate::ir::CpsExpr;
use crate::util::cps::make_let;
use crate::util::syn::ident_pat;
use crate::{
    MacroAllowlist, Result,
    cps::expr::CpsBind,
    cps::{CpsContinuation, CpsStep},
    error::{CpsError, assert_no_attrs},
    ir::CpsExprMacro,
};
use syn::{ExprMacro, Ident, Macro};

impl CpsStep for ExprMacro {
    /// Emit `mac!(...);` as a side-effect statement then continue.
    fn then(self, continuation: CpsContinuation, ctx: &mut CpsCtx) -> Result<CpsContinuation> {
        let mac = assert_expr_macro(self, ctx)?;
        Ok(CpsExpr::from(mac) + continuation)
    }
}

impl CpsBind for ExprMacro {
    /// Emit `let bind = mac!(...);` and continue.
    fn and_then(
        self,
        bind: Ident,
        continuation: CpsContinuation,
        ctx: &mut CpsCtx,
    ) -> Result<CpsContinuation> {
        let mac = assert_expr_macro(self, ctx)?;
        let stmt = make_let(ident_pat(bind), mac);
        Ok(stmt + continuation)
    }
}

#[inline]
pub fn assert_expr_macro(e: ExprMacro, ctx: &mut CpsCtx) -> Result<CpsExprMacro> {
    assert_no_attrs(e.attrs)?;
    let mac = assert_valid_macro(e.mac, ctx)?;
    Ok(CpsExprMacro { mac })
}

#[inline]
pub(crate) fn assert_valid_macro(mac: Macro, ctx: &mut CpsCtx) -> Result<Macro> {
    if !ctx.config.macro_allowlist.is_allowed(&mac) {
        Err(CpsError::UnallowedMacro(mac).into())
    } else {
        Ok(mac)
    }
}

impl MacroAllowlist {
    /// Returns whether the specified macro is allowed under this policy.
    pub fn is_allowed(&self, mac: &Macro) -> bool {
        match self {
            Self::Default => is_known_pure_macro(mac),
            Self::None => false,
            Self::All => true,
            Self::Custom(set) => is_known_pure_macro(mac) || set.contains(&mac.path),
        }
    }
}

/// Checks if a macro is a standard Rust macro known not to perform early jumps/returns.
pub fn is_known_pure_macro(mac: &Macro) -> bool {
    if let Some(ident) = mac.path.get_ident() {
        matches!(
            ident.to_string().as_str(),
            "println"
                | "print"
                | "eprintln"
                | "eprint"
                | "format"
                | "format_args"
                | "vec"
                | "dbg"
                | "write"
                | "writeln"
                | "panic"
                | "unreachable"
                | "todo"
                | "unimplemented"
                | "assert"
                | "assert_eq"
                | "assert_ne"
                | "matches"
                | "concat"
                | "env"
                | "option_env"
                | "stringify"
                | "file"
                | "line"
                | "column"
                | "module_path"
        )
    } else if let Some(last) = mac.path.segments.last() {
        matches!(
            last.ident.to_string().as_str(),
            "println"
                | "print"
                | "eprintln"
                | "eprint"
                | "format"
                | "vec"
                | "dbg"
                | "panic"
                | "unreachable"
                | "todo"
                | "assert"
                | "assert_eq"
                | "assert_ne"
        )
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Config;
    use crate::cps::CpsCtx;
    use syn::parse_quote;

    #[test]
    fn unallowed_macro_is_rejected() {
        let expr: ExprMacro = parse_quote!(unknown_macro!());
        let err = expr.step().unwrap_err();
        assert!(matches!(*err, CpsError::UnallowedMacro(..)));
    }

    #[test]
    fn macro_with_attrs_rejected() {
        let mut expr: ExprMacro = parse_quote!(println!("hello"));
        expr.attrs = vec![parse_quote!(#[inline])];
        let err = expr.step().unwrap_err();
        assert!(matches!(*err, CpsError::UnallowedAttributes(..)));
    }

    #[test]
    fn disallow_all_macros_rejects_even_pure_macros() {
        let expr: ExprMacro = parse_quote!(println!("hello"));
        let mut ctx = CpsCtx::new(Config::new().disallow_all_macros());
        let err = expr.step_with(&mut ctx).unwrap_err();
        assert!(matches!(*err, CpsError::UnallowedMacro(..)));
    }
}
