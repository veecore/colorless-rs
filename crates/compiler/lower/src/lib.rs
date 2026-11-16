//! Transforms Rust syntax trees into Continuation-Passing Style (CPS) and A-Normal Form (ANF).
//!
//! In direct-style Rust, control-flow constructs like `.resolve`, `return`, `break`, `continue`,
//! and `?` produce jumps, early exits, or suspension points. This crate lowers those constructs
//! into an explicit intermediate representation where every step passes control to an explicit
//! continuation or control-flow signal.
//!
//! # Organization
//!
//! - [`cps`]: The lowering visitor implementations for Rust expressions, statements, and blocks,
//!   including continuation buffers, loop scope resolution, and ANF sub-expression hoisting.
//! - [`ir`]: The target intermediate representation nodes ([`CpsBody`](ir::CpsBody),
//!   [`CpsBlock`](ir::CpsBlock), [`CpsStmt`](ir::CpsStmt), and [`CpsTerminator`](ir::CpsTerminator))
//!   consumed by downstream code generators.
//! - [`error`]: Compilation errors and diagnostics.
//! - [`util`]: Helpers for AST construction and syntax desugaring.
//!
//! Lowering begins from any type implementing [`CpsStep`] using [`Config::lower`].

pub mod cps;
pub mod error;
pub mod ir;
pub mod util;

use crate::cps::CpsStep;
pub use error::CpsError;
use std::collections::HashSet;
use syn::{Path, parse_quote, parse_str};

pub type Result<T> = core::result::Result<T, Box<CpsError>>;

/// Configuration for the CPS lowering pass.
#[derive(Clone, Debug)]
pub struct Config {
    /// When `true`, a `let`-`else` branch that cannot be proven to diverge is
    /// accepted anyway.  When `false` (the default), such a branch produces
    /// [`CpsError::UnclearDivergence`].
    pub allow_maybe_diverging_in_let_else: bool,
    /// Allowlist policy controlling which macro invocations are permitted.
    pub macro_allowlist: MacroAllowlist,
    /// Runtime path used by the structured loop-signal encoding.
    pub signal_path: Path,
    /// Runtime path used by the `?` operator desugaring.
    pub try_path: Path,
    /// When `true`, the lowering automatically injects loop-state bookkeeping
    /// for `for` loops.
    pub auto_inject_loop_state: bool,
    /// Path to the `IntoColoredIterator::into_iterator` entry point used when
    /// lowering `for` loops.
    pub colorless_into_iterator: Path,
}

/// Specifies which macro invocations are permitted in CPS-lowered code.
#[derive(Clone, Debug, Default)]
pub enum MacroAllowlist {
    /// Uses the built-in list of known-pure standard library macros (`println!`, `format!`, `vec!`, `panic!`, etc.).
    /// This mode requires zero heap allocations.
    #[default]
    Default,
    /// Disallows all macro invocations.
    None,
    /// Permissive mode: allows any macro invocation without checking.
    All,
    /// Custom allowlist augmenting the default standard macros with user-specified macro names.
    Custom(HashSet<Path>),
}

impl Default for Config {
    fn default() -> Self {
        Self {
            allow_maybe_diverging_in_let_else: false,
            macro_allowlist: Default::default(),
            signal_path: syn::parse_quote!(::cps_signal),
            try_path: syn::parse_quote!(::cps_try),
            auto_inject_loop_state: true,
            colorless_into_iterator: parse_quote!(
                ::colorless::iter::IntoColoredIterator::into_iterator
            ),
        }
    }
}

impl Config {
    /// Create a default config with the built-in standard macro allowlist.
    pub fn new() -> Self {
        Self::default()
    }
    /// Set a custom macro allowlist policy.
    pub fn with_macro_allowlist(mut self, allowlist: MacroAllowlist) -> Self {
        self.macro_allowlist = allowlist;
        self
    }

    /// Allow all macro invocations unconditionally.
    pub fn allow_all_macros(mut self) -> Self {
        self.macro_allowlist = MacroAllowlist::All;
        self
    }

    /// Add a specific macro name to the allowlist.
    pub fn allow_macro_named(mut self, name: impl Into<Path>) -> Self {
        match &mut self.macro_allowlist {
            MacroAllowlist::Custom(set) => {
                set.insert(name.into());
            }
            _ => {
                let mut set = HashSet::new();
                set.insert(name.into());
                self.macro_allowlist = MacroAllowlist::Custom(set);
            }
        }
        self
    }

    pub fn allow_macro_named_str(self, name: &str) -> Self {
        self.allow_macro_named(
            parse_str::<Path>(name)
                .expect("allow_macro_named_str: argument is not a valid Rust path"),
        )
    }

    /// Disallow all macros.
    pub fn disallow_all_macros(mut self) -> Self {
        self.macro_allowlist = MacroAllowlist::None;
        self
    }

    /// Override the loop-signal runtime path used in generated IR.
    ///
    /// Standalone lowering defaults to `::cps_signal`; facade crates can point
    /// this at a public re-export so downstream users need only one dependency.
    pub fn with_signal_path(mut self, path: Path) -> Self {
        self.signal_path = path;
        self
    }

    #[inline]
    pub fn lower<T, S>(self, s: S) -> Result<T>
    where
        S: CpsStep<T>,
    {
        s.step_with(self)
    }
}
