//! Structured loop lowering and signal pattern helpers.
//!
//! Submodules:
//! - [`loop_`]: Infinite `loop { ... }`.
//! - [`while_`]: `while cond { ... }`.
//! - [`for_loop`]: `for pat in iter { ... }`.

pub mod for_loop;
pub mod loop_;
pub mod while_;

use crate::{
    cps::context::loop_::depth::CpsLoopDepth,
    util::syn::{ident_pat, make_syn_call, tuple_struct_pat_path},
};
use proc_macro2::Ident;
use quote::format_ident;
use syn::{Expr, Pat, Path, PathArguments, PathSegment};

pub fn break_signal(runtime: &Path, from: CpsLoopDepth, target: CpsLoopDepth, val: Expr) -> Expr {
    make_syn_call(
        path_member(
            &path_member(runtime, &format!("L{from}Signal")),
            &format!("break_{target}"),
        ),
        [val],
    )
}

pub fn continue_signal(
    runtime: &Path,
    from: CpsLoopDepth,
    target: CpsLoopDepth,
    val: Expr,
) -> Expr {
    make_syn_call(
        path_member(
            &path_member(runtime, &format!("L{from}Signal")),
            &format!("continue_{target}"),
        ),
        [val],
    )
}

pub fn return_signal(runtime: &Path, from: CpsLoopDepth, val: Expr) -> Expr {
    make_syn_call(
        path_member(&path_member(runtime, &format!("L{from}Signal")), "return_"),
        [val],
    )
}

pub fn make_break_local_pat(signal_type: &Path, inner: Pat) -> Pat {
    tuple_struct_pat_path(path_member(signal_type, "Break"), vec![inner])
}

pub fn make_signal_outer_pat(signal_type: &Path, inner: Ident) -> Pat {
    tuple_struct_pat_path(path_member(signal_type, "Outer"), vec![ident_pat(inner)])
}

pub fn make_continue_parent_pat(signal_type: &Path, inner: Ident) -> Pat {
    tuple_struct_pat_path(
        path_member(signal_type, "ContinueParent"),
        vec![ident_pat(inner)],
    )
}

pub(crate) fn path_member(base: &Path, member: &str) -> Path {
    let mut path = base.clone();
    path.segments.push(PathSegment {
        ident: format_ident!("{member}"),
        arguments: PathArguments::None,
    });
    path
}
