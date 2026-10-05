//! # Expression Lowering Pipeline
//!
//! Categorized expression transformations:
//! - [`leaf`]: Atomic leaf expressions that never diverge (Literals, Paths, Idents).
//! - [`compound`]: Pure ANF compound expressions with hoisted sub-expressions (Calls, Arrays, Tuples...).
//! - [`branch`]: Control-flow branching (`ExprIf`, `ExprMatch`).
//! - [`loops`]: Structured loop lowerings (`loop`, `while`, `for`) and signal pattern generation.
//! - [`jumps`]: Non-local control flow exits (`break`, `continue`, `return`, `try ?`).
//! - [`block`]: Labeled, plain, and unsafe blocks.
//! - [`macros`]: Expression lowering DSL (`impl_cps_leaf!`, `impl_cps_hoist!`, `impl_cps_branch!`).

#[macro_use]
pub mod macros;

pub mod block;
pub mod branch;
pub mod compound;
pub mod jumps;
pub mod leaf;
pub mod loops;

use crate::Result;
use crate::cps::continuation::CpsContinuation;
use crate::cps::context::CpsCtx;
use crate::cps::CpsStep;
use proc_macro2::Ident;

/// Value-binding continuation step for an expression.
pub trait CpsBind<T = CpsContinuation>: CpsStep<T> {
    /// Binds `self`'s computed value to identifier `bind`, then proceeds with `continuation`.
    fn and_then(self, bind: Ident, continuation: CpsContinuation, ctx: &mut CpsCtx) -> Result<T>;
}

/// The target continuation of an active labeled non-loop block or branch.
#[derive(Clone, Debug)]
pub enum ContinuationTarget {
    Bind { bind: Ident, continuation: CpsContinuation },
    Step { continuation: CpsContinuation },
}
