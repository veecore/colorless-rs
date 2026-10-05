//! Errors returned during CPS lowering and IR conversion.

/// Error returned when encountering unlowered jumps, unsupported control flow,
/// or invalid syntax during CPS transformation.
#[derive(Debug, Clone)]
pub enum CpsError {
    UnsupportedExpr(&'static str),
    MissingElseBranch,
    Custom(alloc::string::String),
}

extern crate alloc;
