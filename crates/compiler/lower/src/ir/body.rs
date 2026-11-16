use crate::ir::CpsPlainStmt;
use crate::ir::stmt::CpsStmt;
use crate::ir::terminator::CpsTerminator;
use syn::token;

// Maybe put this in arc
/// An unbraced sequence of lowered statements and its structural terminator.
///
/// Continuations and continuation groups contain bodies. Syntax constructs
/// which own braces contain [`CpsBlock`] instead.
#[derive(Clone, Debug)]
pub struct CpsBody {
    pub stmts: Vec<CpsStmt>,
    pub terminator: Option<Box<CpsTerminator>>,
}

/// A block containing lowered CPS statements and an optional terminator.
#[derive(Clone, Debug)]
pub struct CpsBlock {
    pub brace_token: token::Brace,
    pub body: CpsBody,
}

// Codegen would still need to wrap continuations in thunks but this
// helps avoid issues with drop before the thunk is thrown out
// like in this code
//
// {
//      // likely resolve point
//      fetch().resolve;
//      // l references m
//      let l = m.lock().unwrap();
// }
// // cont that uses m
// m.lock()
//
// codegen would generate something like this without block
// let thunk = C::map(fetch(), move || {
//      let l = m.lock().unwrap();
//      move || m.lock()
// });
//
// thunk();
//
// even though this m.lock() would be called outside the block,
// rust sees that l would be dropped at the invisible line below
// returning the thunk which makes it wrong for m (that is referenced
// by l) to be moved into the thunk... so we need to make it clear
// where we want the drop.
//
// But what if the line(s) close to the thunk were resolve points? Like
//
// {
//      // likely resolve point
//      fetch().resolve;
//      // l references m
//      let l = m.lock().resolve.unwrap();
// }
// // cont that uses m
// m.lock()
//
// This is a particular case of local-borrow... imagine the resolve were await
// m.lock() returned a future references m... this future needs to continue
// referencing m while m is alive, which means async needs to keep the storage
// 'm' as well as the future - resulting in needing it pinned... imagine while 'm'
// is kept and the future is polled, one moved the future somewhere else, the pointer
// inside the m.lock() future immediately becomes invalid. So the point is; we cannot
// put the resolve part in block just like we do plain code and that shouldn't be the
// cause of a problem that didn't already exist - fingers crossed
//
// Let's consider another case where the reference is above the fetch()
//
// {
//      // l references m
//      let l = m.lock().unwrap();
//      // likely resolve point
//      fetch().resolve;
// }
// // cont that uses m
// m.lock()
//
// Just like the former case, without the block, this would still be an issue
// Because while fetch() resolves, async needs to keep both l and m alive and pinned
// which is already the local-borrow issue once again
//
// Actually, I just tested with the current code and the above passes... I believe this is
// wrong... this is the keep_alive + local_borrow issue... if the fetch() function somehow
// implicitly accesses the lock, it'd be a deadlock like it should be in native code, but
// in our current async code, it'd just access it since l was dropped earlier - which is
// wrong
//
// Anyway, we have no solution for that in here... we can't wrap the plain parts before the
// resolve because then the continuation would have no access to intermediate variables
#[derive(Clone, Debug)]
pub struct CpsPlainBlock {
    pub brace_token: token::Brace,
    pub stmts: Vec<CpsPlainStmt>,
}

mod convert_impls {
    use crate::{
        ir::body::{CpsBlock, CpsBody},
        ir::{CpsPlainBlock, IntoSyn},
    };
    use syn::{Block, Stmt};

    impl IntoSyn<Vec<Stmt>> for CpsBody {
        #[inline]
        fn into_syn(self) -> Vec<Stmt> {
            let mut stmts = self.stmts.into_syn();
            if let Some(terminator) = self.terminator {
                stmts.extend((*terminator).into_syn());
            }
            stmts
        }
    }

    impl IntoSyn<Block> for CpsBlock {
        #[inline]
        fn into_syn(self) -> Block {
            Block {
                brace_token: self.brace_token,
                stmts: self.body.into_syn(),
            }
        }
    }

    impl IntoSyn<Block> for CpsPlainBlock {
        fn into_syn(self) -> Block {
            Block {
                brace_token: self.brace_token,
                stmts: self.stmts.into_syn(),
            }
        }
    }
}

mod impls {
    use crate::cps::CpsContinuation;
    use crate::ir::body::CpsBody;
    use crate::ir::terminator::CpsTerminator;
    use crate::ir::{CpsBlock, CpsExpr, CpsPlainStmt, CpsStmt, CpsStmtExpr};

    impl From<CpsExpr> for CpsBody {
        fn from(value: CpsExpr) -> Self {
            Self {
                stmts: vec![CpsStmt::Plain(
                    CpsPlainStmt::Expr(CpsStmtExpr {
                        expr: value,
                        semi: None,
                    })
                    .into(),
                )],
                terminator: None,
            }
        }
    }

    impl From<Vec<CpsStmt>> for CpsBody {
        fn from(value: Vec<CpsStmt>) -> Self {
            Self {
                stmts: value,
                terminator: None,
            }
        }
    }

    impl From<CpsTerminator> for CpsBody {
        #[inline]
        fn from(value: CpsTerminator) -> Self {
            Self {
                stmts: vec![],
                terminator: Some(Box::new(value)),
            }
        }
    }

    impl From<CpsBody> for CpsBlock {
        fn from(value: CpsBody) -> Self {
            Self {
                brace_token: Default::default(),
                body: value,
            }
        }
    }

    impl From<CpsContinuation> for CpsBlock {
        #[inline]
        fn from(body: CpsContinuation) -> Self {
            Self {
                brace_token: Default::default(),
                body: body.into(),
            }
        }
    }
}
