//! Structured loop lowering and signal pattern helpers.
//!
//! Submodules:
//! - [`loop_`]: Infinite `loop { ... }`.
//! - [`while_`]: `while cond { ... }`.
//! - [`for_loop`]: `for pat in iter { ... }`.

pub mod for_loop;
pub mod loop_;
pub mod while_;
