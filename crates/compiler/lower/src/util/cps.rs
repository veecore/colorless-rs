//! Zero-macro typed AST builders and helper factories.

use crate::{
    ir::body::CpsBlock,
    ir::expr::{CpsExpr, CpsExprPath, CpsExprTuple, CpsInert},
    ir::scoped::{CpsArm, CpsScoped, CpsScopedMatch},
    ir::stmt::{CpsLocalInit, CpsStmt, CpsStmtExpr, CpsStmtLocal},
    ir::{CpsExprBlock, CpsPlainBlock, CpsPlainStmt},
    util::syn::{ident_pat, tuple_struct_pat},
};
use syn::punctuated::Punctuated;
use syn::*;

/// Creates a path expression from a simple identifier name.
#[inline]
#[must_use]
pub(crate) fn ident_expr(id: Ident) -> CpsExpr {
    CpsExpr::Inert(ident_inert(id))
}

#[inline]
#[must_use]
pub fn ident_inert(id: Ident) -> CpsInert {
    CpsInert::Path(CpsExprPath {
        qself: None,
        path: Path::from(id),
    })
}

/// Creates a unit expression `()`.
#[inline]
#[must_use]
pub(crate) fn unit_expr() -> CpsExpr {
    CpsExpr::Inert(unit_inert())
}

/// Creates a unit inert `()`.
#[inline]
#[must_use]
pub(crate) fn unit_inert() -> CpsInert {
    CpsInert::Tuple(CpsExprTuple {
        paren_token: token::Paren::default(),
        elems: Punctuated::new(),
    })
}

/// Creates a `ControlFlow::Continue(pat)` pattern.
#[inline]
#[must_use]
pub(crate) fn control_flow_continue_pat(inner: Ident) -> Pat {
    tuple_struct_pat("::core::ops::ControlFlow::Continue", vec![ident_pat(inner)])
}

/// Creates a `ControlFlow::Break(pat)` pattern.
#[inline]
#[must_use]
pub(crate) fn control_flow_break_pat(inner: Ident) -> Pat {
    tuple_struct_pat("::core::ops::ControlFlow::Break", vec![ident_pat(inner)])
}

/// Creates a binding statement: `let bind = application;`.
#[inline]
#[must_use]
pub(crate) fn make_bind(bind: Ident, application: impl Into<CpsExpr>) -> CpsStmt {
    make_let(ident_pat(bind), application)
}

/// Creates a `CpsLocal`: `let pat = init;`.
#[inline]
#[must_use]
pub(crate) fn make_local(pat: Pat, init: impl Into<CpsExpr>) -> CpsStmtLocal {
    CpsStmtLocal {
        let_token: token::Let::default(),
        modifiers: Default::default(),
        pat,
        init: Some(CpsLocalInit {
            eq_token: token::Eq::default(),
            expr: Box::new(init.into()),
        }),
        semi_token: token::Semi::default(),
    }
}

/// Creates a `let pat = init;` statement.
#[inline]
#[must_use]
pub(crate) fn make_let(pat: Pat, init: impl Into<CpsExpr>) -> CpsStmt {
    CpsStmt::Plain(Box::new(make_local(pat, init).into()))
}

/// Creates a match arm: `pat => body,`.
#[inline]
#[must_use]
pub(crate) fn make_arm(pat: Pat, body: impl Into<CpsBlock>) -> CpsArm {
    CpsArm {
        pat,
        fat_arrow_token: token::FatArrow::default(),
        body: body.into(),
        comma: Some(token::Comma::default()),
    }
}

#[inline]
#[must_use]
pub(crate) fn make_match(expr: impl Into<Box<CpsExpr>>, arms: Vec<CpsArm>) -> CpsScoped {
    CpsScoped::Match(make_match_(expr, arms))
}

#[inline]
#[must_use]
pub(crate) fn make_match_(expr: impl Into<Box<CpsExpr>>, arms: Vec<CpsArm>) -> CpsScopedMatch {
    CpsScopedMatch {
        match_token: token::Match::default(),
        expr: expr.into(),
        brace_token: token::Brace::default(),
        arms,
    }
}

#[inline]
#[must_use]
pub fn make_stmt(expr: impl Into<CpsExpr>) -> CpsStmt {
    CpsStmt::Plain(Box::new(
        CpsStmtExpr {
            expr: expr.into(),
            semi: Some(token::Semi::default()),
        }
        .into(),
    ))
}

#[inline]
#[must_use]
pub(crate) fn make_unterminated_stmt(expr: impl Into<CpsExpr>) -> CpsStmt {
    CpsStmt::Plain(Box::new(
        CpsStmtExpr {
            expr: expr.into(),
            semi: None,
        }
        .into(),
    ))
}

#[inline]
#[must_use]
pub(crate) fn make_block_stmts(stmts: impl IntoIterator<Item = CpsPlainStmt>) -> CpsExpr {
    CpsExpr::Block(CpsExprBlock {
        block: CpsPlainBlock {
            brace_token: Default::default(),
            stmts: stmts.into_iter().collect(),
        },
    })
}
