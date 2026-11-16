use crate::cps::context::CpsLoopFacts;
use crate::cps::context::loop_::depth::CpsLoopDepth;
use proc_macro2::Ident;
use quote::format_ident;

pub(crate) mod depth;
pub(crate) mod effects;

impl CpsLoopFacts {
    pub(crate) fn signal_binding(&self) -> Ident {
        format_ident!(
            "__loop_signal_{}_{}",
            self.loop_id.depth.0,
            self.loop_id.index
        )
    }

    pub(crate) fn depth(&self) -> CpsLoopDepth {
        self.loop_id.depth
    }
}
