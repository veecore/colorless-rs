//! # `colorless-cps`
//!
//! A compiler-grade library for transforming Rust `syn` syntax trees into
//! **Continuation-Passing Style (CPS)** and **A-Normal Form (ANF)**.
//!
//! ## Core Subsystems
//!
//! - [`ir`]: The jump-free, label-free Intermediate Representation and declaration DSL.
//! - [`cps`]: The lowering engine (continuation buffers, ANF hoister, scopes, expression lowerers).
//! - [`util`]: Shared AST construction utilities.
//! - [`error`]: Compilation errors and diagnostics.

pub mod error;
pub mod ir;
pub mod cps;
pub mod util;

pub use error::CpsError;
pub type Result<T> = core::result::Result<T, Box<CpsError>>;

/// Configuration for the CPS lowering pass.
#[derive(Clone, Debug, Default)]
pub struct Config {
    pub forbid_unsafe: bool,
}
