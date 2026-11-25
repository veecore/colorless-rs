//! The main transpiler
//! The macros would depend on this so they can have their ui alone

use crate::context::FoldContext;
use colorless_lower::Config as LoweringConfig;
pub use colorless_lower::MacroAllowlist;
use colorless_lower::cps::CpsStep;
use colorless_lower::ir::CpsBlock;
use std::mem;
use syn::{Block, Expr, Path, Type, parse_quote};

pub mod context;
pub mod ir;
pub mod rules;
mod runtime_call;
mod util;

pub type Result<T> = std::result::Result<T, syn::Error>;

pub struct Config {
    pub max_one_of_arms: usize,
    pub color_type: Type,
    pub crate_path: Path,
    pub original_block_plain: Option<Block>,
    pub declare_borrow_macros: bool,
    pub no_capture: bool,
    pub no_wrap: bool,
    pub colorless_into_iterator: Path,
    pub desugar_resolve: bool,
    pub erase: bool,
    /// Extra macros to allow beyond the built-in allowlist.
    /// `None` means keep the built-in default; `All` means allow everything;
    /// `Custom` means augment the built-in list with explicit paths.
    pub extra_macro_allowlist: MacroAllowlist,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            max_one_of_arms: 12,
            color_type: parse_quote!(__C),
            crate_path: parse_quote!(::colorless::__compiler),
            original_block_plain: None,
            declare_borrow_macros: true,
            no_capture: false,
            no_wrap: false,
            colorless_into_iterator: parse_quote!(
                ::colorless::iter::IntoColoredIterator::into_iter
            ),
            desugar_resolve: true,
            erase: false,
            extra_macro_allowlist: MacroAllowlist::Default,
        }
    }
}

/// Delegated folding operation used by every CPS node consumed by Colorless.
pub trait Fold<Below = ()> {
    type Output;

    fn fold(self, _below: Below, ctx: &mut FoldContext) -> Result<Self::Output>;
}

impl Config {
    pub fn generate_block(&mut self, block: Block) -> Result<Expr> {
        let mut config = LoweringConfig {
            allow_maybe_diverging_in_let_else: true,
            macro_allowlist: Default::default(),
            signal_path: self.crate_path.clone(),
            try_path: self.crate_path.clone(),
            auto_inject_loop_state: true,
            colorless_into_iterator: self.colorless_into_iterator.clone(),
        };

        config = config.allow_macro_named_str("borrow");
        config = config.allow_macro_named_str("rb");
        config = config.allow_macro_named_str("unborrow");
        config = config.allow_macro_named_str("keep_alive");
        // fixme: shadow and doc that it can't control the outer environment
        //  natively
        config = config.allow_macro_named_str("color");
        config = config.allow_macro_named_str("slot");
        config = config.allow_macro_named_str("take_slot");
        config = config.allow_macro_named_str("rbv");

        // Apply user-specified extra allowlist on top of the built-ins.
        match &self.extra_macro_allowlist {
            MacroAllowlist::All => {
                config = config.allow_all_macros();
            }
            MacroAllowlist::Custom(paths) => {
                for path in paths {
                    config = config.allow_macro_named(path.clone());
                }
            }
            MacroAllowlist::Default | MacroAllowlist::None => {}
        }

        self.generate_from_lowered(block.step_with(config)?)
    }

    pub fn generate_from_lowered(&mut self, lowered: CpsBlock) -> Result<Expr> {
        let c = mem::take(self);
        let mut ctx = FoldContext::new(c);
        let folded = lowered.fold((), &mut ctx)?;
        let r = folded.finish(&mut ctx)?;
        *self = ctx.config;
        Ok(r)
    }
}
