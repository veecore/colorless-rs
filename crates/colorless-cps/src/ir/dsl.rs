//! # IR Declaration DSL
//!
//! Macros for declaring jump-free CPS IR nodes and generating bidirectional `syn` conversions:
//! - `ir_struct!`: Defines an IR node with exact token preservation and `IntoSyn` generation.
//! - `ir_enum!`: Defines an IR sum type and delegates `IntoSyn` dispatch.

#[macro_export]
macro_rules! ir_struct {
    ($($tokens:tt)*) => {};
}

#[macro_export]
macro_rules! ir_enum {
    ($($tokens:tt)*) => {};
}
