pub mod conditional;
pub mod exhaustive;
mod factory;
mod operations;

use std::collections::BTreeSet;

use conditional::ConditionalDataMap;
use exhaustive::{ExhaustiveCallback, ExhaustiveDepId};
pub use factory::Factory;
use oxc::{
  allocator::Allocator,
  span::{GetSpan, Span},
};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
  TreeShakeConfig,
  builtins::Builtins,
  dep::ReferredDeps,
  folding::ConstantFolder,
  mangling::Mangler,
  module::{ModuleId, Modules},
  scope::Scoping,
  utils::ExtraData,
  vfs::Vfs,
};

pub struct Analyzer<'a> {
  pub vfs: Box<dyn Vfs>,
  pub config: &'a TreeShakeConfig,
  pub allocator: &'a Allocator,
  pub factory: &'a Factory<'a>,

  pub modules: Modules<'a>,
  pub builtins: Builtins<'a>,

  pub module_stack: Vec<ModuleId>,
  pub span_stack: Vec<Span>,
  pub scoping: Scoping<'a>,

  pub data: ExtraData<'a>,
  pub exhaustive_callbacks: FxHashMap<ExhaustiveDepId<'a>, FxHashSet<ExhaustiveCallback<'a>>>,
  pub referred_deps: ReferredDeps,
  pub conditional_data: ConditionalDataMap<'a>,
  // pub loop_data: LoopDataMap<'a>,
  pub folder: ConstantFolder<'a>,
  pub mangler: Mangler<'a>,
  pub pending_deps: FxHashSet<ExhaustiveCallback<'a>>,
  pub diagnostics: BTreeSet<String>,
}

impl<'a> Analyzer<'a> {
  pub fn new_in(vfs: Box<dyn Vfs>, config: &'a TreeShakeConfig, allocator: &'a Allocator) -> Self {
    let factory = &*allocator.alloc(Factory::new(allocator, config));
    Analyzer {
      vfs,
      config,
      allocator,
      factory,

      modules: Modules::default(),
      builtins: Builtins::new(config, factory),

      module_stack: vec![],
      span_stack: vec![],
      scoping: Scoping::new(factory),

      data: Default::default(),
      exhaustive_callbacks: Default::default(),
      referred_deps: Default::default(),
      conditional_data: Default::default(),
      // loop_data: Default::default(),
      folder: Default::default(),
      mangler: Mangler::new(config.mangling.is_some(), allocator),
      pending_deps: Default::default(),
      diagnostics: Default::default(),
    }
  }

  pub fn finalize(&mut self) {
    self.module_stack.push(ModuleId::new(0));

    self.consume_exports(ModuleId::new(0));

    let mut round = 0usize;
    loop {
      round += 1;
      if round > 1000 {
        panic!("Possible infinite loop in post analysis");
      }

      let mut dirty = false;
      dirty |= self.call_exhaustive_callbacks();
      dirty |= self.post_analyze_handle_conditional();
      // dirty |= self.post_analyze_handle_loops();
      dirty |= self.post_analyze_handle_folding();
      if !dirty {
        break;
      }
    }

    self.module_stack.pop();

    #[cfg(feature = "flame")]
    {
      self.scoping.call.pop().unwrap().scope_guard.end();
      flamescope::dump(&mut std::fs::File::create("flamescope.json").unwrap()).unwrap();
    }
  }

  pub fn throw_builtin_error(&mut self, message: impl Into<String>) {
    if self.exit_by_throw(false) == 0 {
      self.add_diagnostic(message);
    }
  }

  pub fn add_diagnostic(&mut self, message: impl Into<String>) {
    if !self.span_stack.is_empty() {
      self.diagnostics.insert(format!("{} at {}", message.into(), self.format_current_span()));
    } else {
      self.diagnostics.insert(message.into());
    }
  }

  pub fn current_module(&self) -> ModuleId {
    *self.module_stack.last().unwrap()
  }

  pub fn current_span(&self) -> Span {
    *self.span_stack.last().unwrap()
  }

  pub fn format_current_span(&self) -> String {
    let path = self.module_info().path;
    let span = self.current_span();
    let start = self.line_index().line_col(span.start.into());
    let end = self.line_index().line_col(span.end.into());
    format!("{}:{}:{}-{}:{}", path, start.line + 1, start.col + 1, end.line + 1, end.col + 1)
  }

  pub fn push_span(&mut self, node: &impl GetSpan) {
    self.span_stack.push(node.span());
  }

  pub fn pop_span(&mut self) {
    self.span_stack.pop();
  }
}
