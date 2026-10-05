//! Mutable lowering context carried through CPS transformations.
//!
//! Submodules:
//! - [`scope`]: Target scope frames (`LoopTarget`, `BlockTarget`, `LoopDepth`).
//! - [`resolve`]: Target resolution and upward signal bubbling algorithms.

pub mod resolve;
pub mod scope;

pub use scope::*;
pub use resolve::*;

use crate::Config;
use proc_macro2::Ident;
use quote::format_ident;

/// The compiler context carried through the CPS lowering pass.
pub struct CpsCtx {
    pub config: Config,
    next_temp_id: usize,
}

impl CpsCtx {
    #[inline]
    pub fn new(config: Config) -> Self {
        Self {
            config,
            next_temp_id: 0,
        }
    }

    #[inline]
    pub fn next_temp_ident(&mut self, prefix: &str) -> Ident {
        let id = self.next_temp_id;
        self.next_temp_id += 1;
        format_ident!("__{prefix}_{id}")
    }
}

impl Default for CpsCtx {
    fn default() -> Self {
        Self::new(Config::default())
    }
}
