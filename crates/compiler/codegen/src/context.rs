use crate::{Config, ir::FoldedOutput};
use colorless_lower::cps::context::CpsLoopId;
use colorless_lower::ir::CpsLoopBinding;
use proc_macro2::Ident;
use quote::format_ident;
use std::collections::{BTreeSet, HashMap, HashSet};
use syn::ItemMacro;

pub struct FoldContext {
    pub config: Config,
    continuation_facts: HashMap<Ident, ContinuationFact>,
    // map to the full bindings
    loop_facts: HashMap<CpsLoopId, LoopFact>,
    pub shadow_request: HashSet<Ident>,
    next_ident: usize,
}

pub struct LoopFact {
    pub bindings: BTreeSet<CpsLoopBinding>,
    pub default_break_state: bool,
    pub state_container: ItemMacro,
}

#[derive(Clone)]
pub struct ContinuationFact {
    pub output: FoldedOutput,
    pub container: ContinuationFactContainer,
    pub rehydrate_requests: Vec<Ident>,
}

// we could return inline later for debug
#[derive(Clone)]
pub enum ContinuationFactContainer {
    Macro,
}

impl FoldContext {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            continuation_facts: HashMap::new(),
            loop_facts: Default::default(),
            shadow_request: Default::default(),
            next_ident: 0,
        }
    }

    pub fn store_continuation_fact(&mut self, ident: Ident, fact: ContinuationFact) {
        assert!(
            self.continuation_facts.insert(ident, fact).is_none(),
            "Not already present"
        );
    }

    pub(crate) fn get_continuation_fact(&mut self, ident: &Ident) -> Option<&mut ContinuationFact> {
        self.continuation_facts.get_mut(ident)
    }

    pub fn remove_continuation_fact(&mut self, ident: &Ident) -> Option<ContinuationFact> {
        self.continuation_facts.remove(ident)
    }

    // This should be the full state not just the provided ones
    pub fn store_loop_fact(&mut self, ident: CpsLoopId, fact: LoopFact) {
        assert!(
            self.loop_facts.insert(ident, fact).is_none(),
            "Not already present"
        );
    }

    pub fn get_loop_fact(&self, ident: CpsLoopId) -> Option<&LoopFact> {
        self.loop_facts.get(&ident)
    }

    pub fn remove_loop_fact(&mut self, ident: CpsLoopId) -> Option<LoopFact> {
        self.loop_facts.remove(&ident)
    }

    pub(crate) fn fresh_ident(&mut self, prefix: &str) -> Ident {
        let index = self.next_ident;
        self.next_ident += 1;
        format_ident!("__colorless_{prefix}_{index}")
    }

    pub fn live_continuations(&self) -> usize {
        self.continuation_facts.len()
    }
}
