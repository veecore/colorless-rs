#![cfg(feature = "relaxed-drop")]

//! Source-level contract tests for the public macro surface.
//!
//! Keep these focused on user input and diagnostics. Lower/codegen unit tests
//! exercise IR details; this suite prevents those details from leaking as
//! panics or accidental API changes.

pub mod macro_contract;

#[test]
fn macro_ui() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui/pass/*.rs");
    cases.compile_fail("tests/ui/fail/*.rs");
}
