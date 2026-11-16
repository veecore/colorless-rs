use proc_macro2::Ident;
use std::collections::VecDeque;
use syn::punctuated::Punctuated;
use syn::{
    Block, Expr, ExprBlock, ExprPath, ExprReference, ExprTuple, Item, ItemMacro, Local, LocalInit,
    Pat, PatIdent, PatTuple, PatWild, Path, Stmt, token,
};

#[must_use]
pub fn block(stmts: impl IntoIterator<Item = Stmt>) -> Expr {
    Expr::Block(ExprBlock {
        attrs: vec![],
        label: None,
        block: Block {
            brace_token: Default::default(),
            stmts: stmts.into_iter().collect(),
        },
    })
}

#[must_use]
pub fn stmts(expr: Expr) -> VecDeque<Stmt> {
    VecDeque::from([unterminated_stmt(expr)])
}

pub fn terminated_stmt(expr: Expr) -> Stmt {
    Stmt::Expr(expr, Some(Default::default()))
}

pub fn unterminated_stmt(expr: Expr) -> Stmt {
    Stmt::Expr(expr, None)
}

#[must_use]
pub fn bind_or_discard(bind: Option<Ident>) -> Pat {
    match bind {
        None => Pat::Wild(PatWild {
            attrs: vec![],
            underscore_token: Default::default(),
        }),
        Some(ident) => ident_pat(ident),
    }
}

#[must_use]
pub fn bind(ident: Ident, expr: Expr) -> Stmt {
    let local = Local {
        attrs: vec![],
        let_token: Default::default(),
        modifiers: Default::default(),
        pat: ident_pat(ident),
        init: Some(LocalInit {
            eq_token: Default::default(),
            expr: Box::new(expr),
            diverge: None,
        }),
        semi_token: Default::default(),
    };
    Stmt::Local(local)
}

#[must_use]
pub fn bind_pat(pat: Pat, expr: Expr) -> Stmt {
    let local = Local {
        attrs: vec![],
        let_token: Default::default(),
        modifiers: Default::default(),
        pat,
        init: Some(LocalInit {
            eq_token: Default::default(),
            expr: Box::new(expr),
            diverge: None,
        }),
        semi_token: Default::default(),
    };
    Stmt::Local(local)
}

#[must_use]
pub fn ident_pat(ident: Ident) -> Pat {
    ident_pat_with_mut(false, ident)
}

#[must_use]
pub fn ident_pat_with_mut(_mut: bool, ident: Ident) -> Pat {
    Pat::Ident(PatIdent {
        attrs: vec![],
        by_ref: None,
        mutability: _mut.then(Default::default),
        ident,
        subpat: None,
    })
}

#[must_use]
pub fn ident_expr(ident: Ident) -> Expr {
    expr_path(Path::from(ident))
}

#[must_use]
pub(crate) fn unit() -> Expr {
    Expr::Tuple(ExprTuple {
        attrs: vec![],
        paren_token: token::Paren::default(),
        elems: Punctuated::new(),
    })
}

#[must_use]
pub fn expr_path(path: Path) -> Expr {
    Expr::Path(ExprPath {
        attrs: vec![],
        qself: None,
        path,
    })
}

#[must_use]
pub fn reference(i: Ident) -> Expr {
    Expr::Reference(ExprReference {
        attrs: vec![],
        and_token: Default::default(),
        mutability: None,
        expr: Box::new(ident_expr(i)),
    })
}

#[must_use]
pub fn tuple(exprs: impl IntoIterator<Item = Expr>) -> Expr {
    Expr::Tuple(ExprTuple {
        attrs: vec![],
        paren_token: Default::default(),
        elems: exprs.into_iter().collect(),
    })
}

pub fn tuple_pat(s: impl IntoIterator<Item = Pat>) -> Pat {
    Pat::Tuple(PatTuple {
        attrs: vec![],
        paren_token: Default::default(),
        elems: s.into_iter().collect(),
    })
}

#[must_use]
pub fn macro_(item: ItemMacro) -> Stmt {
    Stmt::Item(Item::Macro(item))
}

pub fn prepend_vecdeque<T>(
    iter: impl DoubleEndedIterator<Item = T>,
    mut v: VecDeque<T>,
) -> VecDeque<T> {
    iter.into_iter().rev().for_each(|r| v.push_front(r));

    v
}
