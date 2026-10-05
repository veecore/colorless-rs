//! # CPS Intermediate Representation (IR)
//!
//! A type-safe intermediate representation guaranteed at compile-time to contain
//! no `return`, `break`, `continue`, or loop labels.
//!
//! Submodules:
//! - [`dsl`]: Declarative macros (`ir_struct!`, `ir_enum!`) for IR definitions.
//! - [`construct`]: Typed AST builders and helper factories.
//! - [`convert`]: Bidirectional conversion between `syn` AST and CPS IR (`IntoCps`, `IntoSyn`).

#[macro_use]
pub mod dsl;
pub mod construct;
pub mod convert;

/// A CPS-lowered statement.
#[derive(Clone, Debug)]
pub enum CpsStmt {
    Expr,
    Local,
}

/// A block containing lowered CPS statements and an optional terminator.
#[derive(Clone, Debug, Default)]
pub struct CpsBlock {
    pub stmts: alloc::vec::Vec<CpsStmt>,
}

extern crate alloc;
