//! Lowering DSL macros for expression nodes:
//! - `impl_cps_leaf!`: For atomic leaves that never diverge.
//! - `impl_cps_hoist!`: For compound expressions requiring ANF hoisting.
//! - `impl_cps_branch!`: For control-flow branching expressions.

#[macro_export]
macro_rules! impl_cps_leaf {
    ($($tokens:tt)*) => {};
}

#[macro_export]
macro_rules! impl_cps_hoist {
    ($($tokens:tt)*) => {};
}

#[macro_export]
macro_rules! impl_cps_branch {
    ($($tokens:tt)*) => {};
}
