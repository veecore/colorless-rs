//! Continuation buffer holding a sequence of lowered statements and an optional terminator.

use crate::{
    cps::CpsCtx,
    ir::CpsPlainStmt,
    ir::CpsScopeId,
    ir::expr::CpsExpr,
    ir::scoped::{CpsScopedCont, CpsScopedContinuation, CpsTerminatorScopedContUse},
    ir::stmt::{CpsStmt, CpsStmtItem, CpsStmtLocal},
    ir::terminator::CpsTerminator,
    ir::{CpsBody, CpsStmtExpr},
    util::cps::{make_stmt, make_unterminated_stmt},
};
use proc_macro2::Ident;
use std::collections::VecDeque;
use std::ops::Add;
use std::rc::Rc;
use syn::Item;

use crate::ir::scoped::{CpsScoped, CpsTerminatorScoped};
#[cfg(test)]
use crate::util::cps::unit_expr;
use crate::util::cps::{ident_expr, make_bind, make_block_stmts};
#[cfg(test)]
use proc_macro2::TokenStream;
#[cfg(test)]
use quote::ToTokens;

#[derive(Clone, Debug, Default)]
pub struct CpsContinuation {
    leading_stmts: LeadingStmts,
    pub(crate) terminator: Option<Box<CpsTerminator>>,
}

#[derive(Clone, Debug)]
enum LeadingStmts {
    CollectPlainBlock {
        binding: Option<Ident>,
        buf: VecDeque<CpsPlainStmt>,
    },
    CollectSequence(VecDeque<CpsStmt>),
}

impl CpsContinuation {
    #[inline]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Creates an empty continuation buffer with the specified capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            leading_stmts: LeadingStmts::with_capacity(capacity),
            terminator: None,
        }
    }

    #[cfg(test)]
    #[inline]
    pub fn unit() -> Self {
        CpsContinuation::from(make_unterminated_stmt(unit_expr()))
    }

    /// Checks whether the continuation contains neither statements nor a terminator.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.leading_stmts.is_empty() && self.terminator.is_none()
    }

    #[inline]
    #[must_use]
    pub fn end_branch_scope(self, binding: Option<Ident>, ctx: &mut CpsCtx) -> ScopeEnd {
        self.end_scope(binding, ctx, true)
    }

    #[inline]
    #[must_use]
    pub fn end_non_branch_scope(self, binding: Option<Ident>, ctx: &mut CpsCtx) -> ScopeEnd {
        self.end_scope(binding, ctx, false)
    }

    /// Group this continuation for one exact structural owner.
    #[inline]
    #[must_use]
    pub fn end_scope(self, binding: Option<Ident>, ctx: &mut CpsCtx, is_branch: bool) -> ScopeEnd {
        if self.is_empty() {
            return ScopeEnd {
                cont: None,
                scope_id: None,
            };
        }

        if let Some(t) = self.as_scope_end() {
            return ScopeEnd {
                cont: Some(t.scope_continuation.clone()),
                scope_id: None,
            };
        }

        let scope_id = ctx.next_scope_ident();
        let group = CpsScopedCont {
            ident: ctx.next_temp_ident("continuation"),
            binding,
            body: self,
            is_branch,
        };
        ScopeEnd {
            cont: Some(Rc::new(group)),
            scope_id: Some(scope_id),
        }
    }

    #[inline]
    pub(crate) fn as_scope_end(&self) -> Option<&CpsTerminatorScopedContUse> {
        if self.leading_stmts.is_empty()
            && let Some(CpsTerminator::ContUse(g)) = self.terminator.as_deref()
        {
            Some(g)
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct ScopeEnd {
    cont: Option<CpsScopedContinuation>,
    scope_id: Option<CpsScopeId>,
}

impl ScopeEnd {
    pub fn continuation(&self) -> Option<&CpsScopedContinuation> {
        self.cont.as_ref()
    }

    pub fn into_terminator(self, scoped: CpsScoped) -> CpsContinuation {
        CpsContinuation::from(CpsTerminator::Scoped(CpsTerminatorScoped {
            scope_id: self.scope_id,
            scoped,
            scope_continuation: if self.scope_id.is_none() {
                None
            } else {
                self.cont
            },
        }))
    }

    /// Returns an empty `ScopeEnd` with no continuation and no scope ID.
    /// Used as a placeholder when the continuation is known to be absent.
    pub fn hole() -> Self {
        Self {
            cont: None,
            scope_id: None,
        }
    }

    /// Wraps an existing scoped continuation in a new `ScopeEnd` with a fresh
    /// scope ID allocated from `ctx`.
    pub fn new_owned(ctx: &mut CpsCtx, cont: CpsScopedContinuation) -> Self {
        Self {
            cont: Some(cont),
            scope_id: Some(ctx.next_scope_ident()),
        }
    }
}

impl LeadingStmts {
    fn with_capacity(capacity: usize) -> Self {
        Self::CollectSequence(VecDeque::with_capacity(capacity))
    }

    fn is_empty(&self) -> bool {
        match self {
            LeadingStmts::CollectPlainBlock { buf, .. } => buf.is_empty(),
            LeadingStmts::CollectSequence(b) => b.is_empty(),
        }
    }

    fn finish_block(mut b: VecDeque<CpsPlainStmt>, binding: Option<Ident>) -> Option<CpsStmt> {
        if b.is_empty() {
            return None;
        }
        Some(match binding {
            None => CpsStmt::Plain(
                CpsPlainStmt::Expr(CpsStmtExpr {
                    expr: make_block_stmts(b),
                    semi: None,
                })
                .into(),
            ),
            Some(binding) => {
                b.push_back(CpsPlainStmt::Expr(CpsStmtExpr {
                    expr: ident_expr(binding.clone()),
                    semi: None,
                }));
                make_bind(binding, make_block_stmts(b))
            }
        })
    }
}

impl Default for LeadingStmts {
    fn default() -> Self {
        Self::CollectSequence(VecDeque::new())
    }
}

impl Add<LeadingStmts> for CpsStmt {
    type Output = LeadingStmts;

    #[inline]
    fn add(self, rhs: LeadingStmts) -> Self::Output {
        match (self, rhs) {
            (CpsStmt::Plain(plain), LeadingStmts::CollectPlainBlock { mut buf, binding }) => {
                buf.push_front(*plain);
                LeadingStmts::CollectPlainBlock { buf, binding }
            }
            (head, LeadingStmts::CollectPlainBlock { buf, binding }) => {
                let mut sequence = VecDeque::from([head]);
                if let Some(tail) = LeadingStmts::finish_block(buf, binding) {
                    sequence.push_back(tail);
                }
                LeadingStmts::CollectSequence(sequence)
            }
            (head, LeadingStmts::CollectSequence(mut b1)) => {
                b1.push_front(head);
                LeadingStmts::CollectSequence(b1)
            }
        }
    }
}

impl From<LeadingStmts> for Vec<CpsStmt> {
    fn from(value: LeadingStmts) -> Self {
        match value {
            LeadingStmts::CollectPlainBlock { buf, binding } => {
                LeadingStmts::finish_block(buf, binding)
                    .into_iter()
                    .collect()
            }
            LeadingStmts::CollectSequence(b) => b.into(),
        }
    }
}

impl Add<CpsContinuation> for CpsStmt {
    type Output = CpsContinuation;

    #[inline]
    fn add(self, mut rhs: CpsContinuation) -> Self::Output {
        rhs.leading_stmts = self + rhs.leading_stmts;
        rhs
    }
}

impl Add<CpsContinuation> for CpsStmtLocal {
    type Output = CpsContinuation;

    #[inline]
    fn add(self, rhs: CpsContinuation) -> Self::Output {
        CpsStmt::Plain(Box::new(self.into())) + rhs
    }
}

impl Add<CpsContinuation> for Item {
    type Output = CpsContinuation;

    #[inline]
    fn add(self, rhs: CpsContinuation) -> Self::Output {
        CpsStmt::Plain(Box::new(CpsStmtItem(self).into())) + rhs
    }
}

impl Add<CpsContinuation> for CpsExpr {
    type Output = CpsContinuation;

    #[inline]
    fn add(self, rhs: CpsContinuation) -> Self::Output {
        if rhs.is_empty() {
            make_unterminated_stmt(self) + rhs
        } else {
            make_stmt(self) + rhs
        }
    }
}

impl From<CpsContinuation> for CpsBody {
    #[inline]
    fn from(value: CpsContinuation) -> Self {
        CpsBody {
            stmts: value.leading_stmts.into(),
            terminator: value.terminator,
        }
    }
}

impl From<CpsStmt> for CpsContinuation {
    #[inline]
    fn from(v: CpsStmt) -> CpsContinuation {
        CpsContinuation {
            leading_stmts: LeadingStmts::CollectSequence(VecDeque::from([v])),
            terminator: None,
        }
    }
}

impl From<CpsTerminator> for CpsContinuation {
    #[inline]
    fn from(d: CpsTerminator) -> Self {
        // A `ContUse` terminator means the preceding statements form a plain
        // block feeding into a scoped continuation, so we start in
        // `CollectPlainBlock` mode and record the binding for the tail.
        // Every other terminator starts in sequence mode.
        let (end_scope, binding) = match &d {
            CpsTerminator::ContUse(CpsTerminatorScopedContUse {
                scope_continuation, ..
            }) => (true, scope_continuation.binding.clone()),
            _ => (false, None),
        };

        CpsContinuation {
            leading_stmts: if end_scope {
                LeadingStmts::CollectPlainBlock {
                    buf: VecDeque::new(),
                    binding,
                }
            } else {
                LeadingStmts::CollectSequence(VecDeque::new())
            },
            terminator: Some(Box::new(d)),
        }
    }
}

impl From<CpsBody> for CpsContinuation {
    fn from(value: CpsBody) -> Self {
        CpsContinuation {
            leading_stmts: LeadingStmts::CollectSequence(value.stmts.into()),
            terminator: value.terminator,
        }
    }
}

#[cfg(test)]
impl ToTokens for CpsContinuation {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        use crate::ir::IntoSyn;

        let body: CpsBody = self.clone().into();
        let stmts = body.into_syn();
        for stmt in stmts {
            stmt.to_tokens(tokens);
        }
    }
}
