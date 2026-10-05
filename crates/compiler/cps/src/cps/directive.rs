//! Parsers for Colorless compiler directives consumed during CPS lowering.
//!
//! Directives (`#[colorless::loop_state(...)]`, `borrow_mut!(...)`, `end_borrow!(...)`)
//! are recognized at this boundary and stripped before emitting CPS IR.
