use crate::{ir::expr::CpsExpr, ir_enum};
use proc_macro2::{Ident, Span};
use std::collections::HashSet;
use syn::{Item, LocalModifiers, Macro, Pat, Token};

ir_enum! {
    /// A CPS-lowered statement.
    #[derive(Clone, Debug)]
    pub enum CpsStmt {
        Directive(pub CpsStmtDirective),
        Plain(pub Box<CpsPlainStmt>),
        Resolve(pub CpsStmtResolve),
    }
}

ir_enum! {
    #[derive(Clone, Debug)]
    pub enum CpsPlainStmt {
        Expr(pub CpsStmtExpr),
        Item(CpsStmtItem),
        Local(CpsStmtLocal),
        Macro(CpsStmtMacro),
    }
}

ir_enum! {
    #[derive(Clone, Debug)]
    pub enum CpsStmtDirective {
        LocalPin(pub CpsStmtDirectiveLocalPin),
        LocalPinEnd(pub CpsStmtDirectiveLocalPinEnd),
    }
}

#[derive(Clone, Debug)]
pub struct CpsStmtExpr {
    pub expr: CpsExpr,
    pub semi: Option<Token![;]>,
}

#[derive(Clone, Debug)]
pub struct CpsStmtItem(pub Item);

#[derive(Clone, Debug)]
pub struct CpsStmtLocal {
    pub let_token: Token![let],
    pub modifiers: LocalModifiers,
    pub pat: Pat,
    pub init: Option<CpsLocalInit>,
    pub semi_token: Token![;],
}

#[derive(Clone, Debug)]
pub struct CpsStmtMacro {
    pub mac: Macro,
    pub semi_token: Option<Token![;]>,
}

#[derive(Clone, Debug)]
pub struct CpsStmtResolve {
    pub bind: Option<Ident>,
    pub base: Box<CpsExpr>,
    pub dot_token: Token![.],
    pub resolve_token: Ident,
    pub semi_token: Option<Token![;]>,
}

#[derive(Clone, Debug)]
pub struct CpsLocalInit {
    pub eq_token: Token![=],
    pub expr: Box<CpsExpr>,
}

#[derive(Clone, Debug)]
pub struct CpsStmtDirectiveLocalPin {
    pub input: CpsLocalPinInput,
}

#[derive(Clone, Debug)]
pub struct CpsStmtDirectiveLocalPinEnd {
    pub input: CpsLocalPinInput,
}

#[derive(Clone, Debug)]
pub struct CpsLocalPinInput {
    pub span: Span,
    pub locals: HashSet<CpsLocalPinTarget>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CpsLocalPinTarget {
    pub ident: Ident,
}

mod convert_impls {
    use crate::{
        derive_into_syn,
        ir::CpsStmtDirective,
        ir::stmt::{
            CpsLocalInit, CpsStmt, CpsStmtExpr, CpsStmtItem, CpsStmtLocal, CpsStmtMacro,
            CpsStmtResolve,
        },
        ir::{CpsPlainStmt, IntoSyn},
        util::syn::{ident_pat, make_bind_syn},
        util::syn::{make_stmt_expr, unit_syn_expr},
    };
    use syn::{Expr, ExprField, Item, Local, LocalInit, Member, Stmt, StmtMacro};

    impl IntoSyn<Stmt> for CpsStmt {
        fn into_syn(self) -> Stmt {
            match self {
                CpsStmt::Directive(v) => v.into_syn(),
                CpsStmt::Plain(v) => (*v).into_syn(),
                CpsStmt::Resolve(v) => v.into_syn(),
            }
        }
    }
    impl IntoSyn<Stmt> for CpsPlainStmt {
        fn into_syn(self) -> Stmt {
            CpsPlainStmt!(self => Stmt => into_syn())
        }
    }

    impl IntoSyn<Stmt> for CpsStmtDirective {
        fn into_syn(self) -> Stmt {
            make_stmt_expr(unit_syn_expr())
        }
    }

    impl IntoSyn<Stmt> for CpsStmtExpr {
        fn into_syn(self) -> Stmt {
            Stmt::Expr(self.expr.into_syn(), self.semi)
        }
    }
    impl IntoSyn<Item> for CpsStmtItem {
        fn into_syn(self) -> Item {
            self.0
        }
    }
    derive_into_syn! {
        Local {
            _ attrs,
            let_token,
            modifiers,
            pat,
            (pub init),
            semi_token,
        } -> {
            CpsStmtLocal
        }
    }
    derive_into_syn! {
        StmtMacro {
            _ attrs,
            (pub mac),
            semi_token,
        } -> {
            CpsStmtMacro
        }
    }
    impl IntoSyn<Stmt> for CpsStmtResolve {
        #[inline]
        fn into_syn(self) -> Stmt {
            let expr = Expr::Field(ExprField {
                attrs: Vec::new(),
                base: self.base.into_syn(),
                dot_token: self.dot_token,
                member: Member::Named(self.resolve_token),
            });

            match self.bind {
                None => Stmt::Expr(expr, self.semi_token),
                Some(bind) => make_bind_syn(ident_pat(bind), expr),
            }
        }
    }

    derive_into_syn! {
        LocalInit {
            eq_token,
            (pub expr),
            _ diverge,
        } -> {
            CpsLocalInit
        }
    }
}
