//! Mutable lowering context carried through CPS transformations.
//!
//! Submodules:
//! - [`query`]: Target scope frames (`LoopTarget`, `BlockTarget`, `LoopDepth`).
//! - [`control_flow`]: Target resolution and upward signal bubbling algorithms.

pub mod control_flow;
pub mod loop_;
pub mod query;

pub use control_flow::*;
pub use query::*;

use crate::Config;
use crate::ir::CpsScopeId;
use proc_macro2::Ident;
use quote::format_ident;
use syn::Path;

/// The compiler context carried through the CPS lowering pass.
#[derive(Debug)]
pub struct CpsCtx {
    pub(crate) config: Config,
    pub(super) control_flow_context: ControlFlowContext,
    /// Monotonically increasing counter for fresh-identifier generation.
    next_temp_id: usize,
    next_scope_id: usize,
}

impl CpsCtx {
    #[inline]
    pub fn new(config: Config) -> Self {
        Self {
            config,
            control_flow_context: ControlFlowContext::default(),
            next_temp_id: 0,
            next_scope_id: 0,
        }
    }

    #[inline]
    #[must_use]
    pub fn next_temp_ident(&mut self, prefix: &str) -> Ident {
        let id = self.next_temp_id;
        self.next_temp_id = self
            .next_temp_id
            .checked_add(1)
            .expect("temporary identifier space exhausted");
        format_ident!("__{prefix}_{id}")
    }

    #[inline]
    #[must_use]
    pub fn next_scope_ident(&mut self) -> CpsScopeId {
        let id = CpsScopeId(self.next_scope_id);
        self.next_scope_id = self
            .next_scope_id
            .checked_add(1)
            .expect("scope identifier space exhausted");
        id
    }

    #[inline]
    pub(crate) fn signal_path(&self) -> &Path {
        &self.config.signal_path
    }
}

impl Default for CpsCtx {
    fn default() -> Self {
        Self::new(Config::default())
    }
}

impl From<Config> for CpsCtx {
    #[inline]
    fn from(config: Config) -> Self {
        Self::new(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_idents_are_unique_within_context() {
        let mut ctx = CpsCtx::default();
        let a = ctx.next_temp_ident("tmp");
        let b = ctx.next_temp_ident("tmp");
        let c = ctx.next_temp_ident("val");
        assert_eq!(a.to_string(), "__tmp_0");
        assert_eq!(b.to_string(), "__tmp_1");
        assert_eq!(c.to_string(), "__val_2");
    }

    #[test]
    fn scopes_receive_unique_opaque_ids() {
        let mut ctx = CpsCtx::default();
        let first = ctx.next_scope_ident();
        let second = ctx.next_scope_ident();

        assert_ne!(first, second);
    }
}
