//! Helper constructors for standard `syn` AST nodes (blocks, paths, break, if/else).
//!
use proc_macro2::{Ident, Span};
use quote::format_ident;
use syn::punctuated::Punctuated;
use syn::{
    Arm, Attribute, Block, Expr, ExprBlock, ExprBreak, ExprCall, ExprContinue, ExprIf, ExprLit,
    ExprMatch, ExprPath, ExprTuple, Lit, LitBool, Local, LocalInit, Pat, PatIdent, PatTupleStruct,
    PatType, PatWild, Path, PathArguments, PathSegment, Stmt, Type, TypePath, token,
};

/// Creates a multi-segment or single-segment path from an identifier string (e.g. `"std::option::Option"`).
#[inline]
#[must_use]
pub fn path_from_str(name: &str) -> Path {
    let mut segments = name.split("::");

    let leading_colon = if name.starts_with("::") {
        segments.next().unwrap();
        Some(token::PathSep::default())
    } else {
        None
    };

    let segments = segments
        .map(|seg| PathSegment {
            ident: format_ident!("{seg}"),
            arguments: PathArguments::None,
        })
        .collect();
    Path {
        leading_colon,
        segments,
    }
}

/// Creates a wildcard pattern `_`.
#[inline]
#[must_use]
pub fn wildcard_pat() -> Pat {
    Pat::Wild(PatWild {
        attrs: vec![],
        underscore_token: token::Underscore::default(),
    })
}

/// Creates a single variable identifier binding pattern (`ident`).
#[inline]
#[must_use]
pub(crate) fn ident_pat(id: Ident) -> Pat {
    Pat::Ident(PatIdent {
        attrs: vec![],
        by_ref: None,
        mutability: None,
        ident: id,
        subpat: None,
    })
}

#[inline]
#[must_use]
pub fn ident_pat_ty_annotate(id: Ident, ty: Path) -> Pat {
    Pat::Type(PatType {
        attrs: vec![],
        pat: ident_pat(id).into(),
        colon_token: Default::default(),
        ty: Box::new(path_ty(ty)),
    })
}

pub fn path_ty(path: Path) -> Type {
    Type::Path(TypePath {
        attrs: vec![],
        qself: None,
        path,
    })
}

/// Creates a `Some(pat)` pattern.
#[inline]
#[must_use]
pub(crate) fn some_pat(inner: Pat) -> Pat {
    tuple_struct_pat("Some", vec![inner])
}

/// Creates a `None` path pattern.
#[inline]
#[must_use]
pub(crate) fn none_pat() -> Pat {
    Pat::Path(ExprPath {
        attrs: vec![],
        qself: None,
        path: path_from_str("None"),
    })
}

/// Creates a mutable variable identifier binding pattern (`mut ident`).
#[inline]
#[must_use]
pub(crate) fn mut_ident_pat(id: Ident) -> Pat {
    Pat::Ident(PatIdent {
        attrs: vec![],
        by_ref: None,
        mutability: Some(token::Mut::default()),
        ident: id,
        subpat: None,
    })
}

/// Creates a tuple struct pattern, e.g. `Some(pat)` or `Ok(pat)`.
#[inline]
#[must_use]
pub(crate) fn tuple_struct_pat(path_str: &str, elems: Vec<Pat>) -> Pat {
    tuple_struct_pat_path(path_from_str(path_str), elems)
}

pub(crate) fn tuple_struct_pat_path(path: Path, elems: Vec<Pat>) -> Pat {
    Pat::TupleStruct(PatTupleStruct {
        attrs: vec![],
        qself: None,
        path,
        paren_token: token::Paren::default(),
        elems: Punctuated::from_iter(elems),
    })
}

#[inline]
#[must_use]
pub(crate) fn make_bind_syn(pat: Pat, expr: Expr) -> Stmt {
    Stmt::Local(Local {
        attrs: Vec::new(),
        let_token: Default::default(),
        modifiers: Default::default(),
        pat,
        init: Some(LocalInit {
            eq_token: Default::default(),
            expr: Box::new(expr),
            diverge: None,
        }),
        semi_token: Default::default(),
    })
}

#[inline]
#[must_use]
pub(crate) fn unit_syn_expr() -> Expr {
    Expr::Tuple(ExprTuple {
        attrs: vec![],
        paren_token: token::Paren::default(),
        elems: Punctuated::new(),
    })
}

#[inline]
#[must_use]
pub(crate) fn make_syn_break(val: Option<Expr>) -> Expr {
    Expr::Break(ExprBreak {
        attrs: vec![],
        break_token: token::Break::default(),
        label: None,
        expr: val.map(Box::new),
    })
}

#[inline]
#[must_use]
pub(crate) fn make_syn_if_else(
    cond: Box<Expr>,
    then_branch: Block,
    else_branch: impl Into<Box<Expr>>,
) -> Expr {
    Expr::If(ExprIf {
        attrs: vec![],
        if_token: Default::default(),
        cond,
        then_branch,
        else_branch: Some((Default::default(), else_branch.into())),
    })
}

pub(crate) fn make_false() -> Expr {
    make_bool(false)
}

pub(crate) fn make_true() -> Expr {
    make_bool(true)
}

pub(crate) fn make_bool(value: bool) -> Expr {
    Expr::Lit(ExprLit {
        attrs: vec![],
        lit: Lit::Bool(LitBool {
            value,
            span: Span::call_site(),
        }),
    })
}

#[inline]
#[must_use]
pub(crate) fn make_syn_if_else_with_attr(
    attrs: Vec<Attribute>,
    cond: Box<Expr>,
    then_branch: Expr,
    else_branch: Expr,
) -> ExprIf {
    ExprIf {
        attrs,
        if_token: Default::default(),
        cond,
        then_branch: make_syn_block(then_branch),
        else_branch: Some((Default::default(), else_branch.into())),
    }
}

#[inline]
#[must_use]
pub(crate) fn make_syn_block(e: Expr) -> Block {
    Block {
        brace_token: Default::default(),
        stmts: vec![Stmt::Expr(e, None)],
    }
}

#[inline]
#[must_use]
pub(crate) fn make_syn_block_stmts(stmts: impl IntoIterator<Item = Stmt>) -> Expr {
    make_syn_expr_block(Block {
        brace_token: Default::default(),
        stmts: stmts.into_iter().collect(),
    })
}

#[inline]
#[must_use]
pub(crate) fn ident_syn_expr(ident: Ident) -> Expr {
    Expr::Path(ExprPath {
        attrs: vec![],
        qself: None,
        path: Path::from(ident),
    })
}

/// Creates a match arm: `pat => body,`.
#[inline]
#[must_use]
pub(crate) fn make_syn_arm(pat: Pat, body: Expr) -> Arm {
    Arm {
        attrs: vec![],
        pat,
        fat_arrow_token: token::FatArrow::default(),
        body: Box::new(body),
        comma: Some(token::Comma::default()),
    }
}

#[inline]
#[must_use]
pub(crate) fn make_syn_none_arm(body: Expr) -> Arm {
    make_syn_arm(none_pat(), body)
}

#[inline]
#[must_use]
pub(crate) fn make_syn_some_arm(inner: Pat, body: Expr) -> Arm {
    make_syn_arm(some_pat(inner), body)
}

#[inline]
#[must_use]
pub(crate) fn make_syn_expr_block(body: Block) -> Expr {
    Expr::Block(ExprBlock {
        attrs: vec![],
        label: None,
        block: body,
    })
}

#[inline]
#[must_use]
pub(crate) fn make_syn_match_(expr: impl Into<Box<Expr>>, arms: impl Into<Vec<Arm>>) -> ExprMatch {
    ExprMatch {
        attrs: vec![],
        match_token: token::Match::default(),
        expr: expr.into(),
        brace_token: token::Brace::default(),
        arms: arms.into(),
    }
}

#[inline]
#[must_use]
pub(crate) fn make_syn_match(expr: impl Into<Box<Expr>>, arms: impl Into<Vec<Arm>>) -> Expr {
    Expr::Match(make_syn_match_(expr, arms))
}

#[inline]
#[must_use]
pub(crate) fn make_syn_call<T>(func: Path, args: impl IntoIterator<Item = T>) -> Expr
where
    Punctuated<Expr, token::Comma>: FromIterator<T>,
{
    Expr::Call(ExprCall {
        attrs: vec![],
        func: path_expr(func).into(),
        paren_token: Default::default(),
        args: Punctuated::from_iter(args),
    })
}

/// Creates a path expression from a string path (e.g. `"Ok"`, `"Option::None"`).
#[inline]
#[must_use]
pub(crate) fn path_expr(path: Path) -> Expr {
    Expr::Path(ExprPath {
        attrs: vec![],
        qself: None,
        path,
    })
}

#[inline]
#[must_use]
pub(crate) fn make_stmt_expr(expr: Expr) -> Stmt {
    Stmt::Expr(expr, Some(Default::default()))
}

#[inline]
#[must_use]
pub(crate) fn make_unterminated_stmt_expr_syn(expr: Expr) -> Stmt {
    Stmt::Expr(expr, None)
}

#[inline]
#[must_use]
pub(crate) fn make_continue() -> Expr {
    Expr::Continue(ExprContinue {
        attrs: vec![],
        continue_token: Default::default(),
        label: None,
    })
}
