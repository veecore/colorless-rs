//! # CPS Lowering Engine
//!
//! Subsystems:
//! - [`continuation`]: The continuation buffer ([`CpsContinuation`]).
//! - [`anf`]: A-Normal Form (ANF) expression hoister ([`anf::Hoister`]).
//! - [`context`]: Scopes, targets, and signal resolution.
//! - [`expr`]: Categorized expression lowering (leaves, compounds, branches, loops, jumps).
//! - [`stmts`]: Statement folding and sequencing.
//! - [`item`]: Item-level syntax lowering (`ItemFn`).
//! - [`directive`]: Compiler directive parsers (`#[loop_state]`, `#[borrow_mut]`).

pub mod anf;
pub mod continuation;
pub mod context;
pub mod directive;
pub mod expr;
pub mod item;
pub mod stmts;

pub use continuation::CpsContinuation;
pub use context::CpsCtx;

use crate::Result;

/// Foundational step in Continuation-Passing Style lowering.
///
/// Sequences `c` after `self`, discarding `self`'s computed value.
pub trait CpsStep<T = CpsContinuation>: Sized {
    fn then(self, c: CpsContinuation, ctx: &mut CpsCtx) -> Result<T>;
}

/// Helper trait to collapse lowered IR into the target syntax.
pub trait CollapseInto<S>: Sized {
    fn collapse_into(self) -> S;
}
