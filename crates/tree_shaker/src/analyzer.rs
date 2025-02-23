use crate::{
  ast::AstKind2,
  builtins::Builtins,
  dep::{DepId, ReferredDeps},
  entity::{Entity, EntityFactory},
  mangling::Mangler,
  scope::{
    conditional::ConditionalDataMap, exhaustive::ExhaustiveCallback, r#loop::LoopDataMap,
    ScopeContext,
  },
  tree_shaker::TreeShaker,
  utils::{ExtraData, StatementVecData},
  TreeShakeConfig,
};
use line_index::LineIndex;
use oxc::{
  allocator::Allocator,
  ast::ast::Program,
  semantic::{Semantic, SymbolId},
  span::{GetSpan, Span},
};
use rustc_hash::FxHashSet;
use std::mem;

pub struct Analyzer<'a> {
  pub tree_shaker: TreeShaker<'a>,
  pub config: &'a TreeShakeConfig,
  pub allocator: &'a Allocator,
  pub factory: &'a EntityFactory<'a>,

  pub line_index: LineIndex,
  pub semantic: Semantic<'a>,
  pub span_stack: Vec<Span>,
  pub data: ExtraData<'a>,
  pub referred_deps: ReferredDeps,
  pub conditional_data: ConditionalDataMap<'a>,
  pub loop_data: LoopDataMap<'a>,
  pub mangler: Mangler<'a>,
  pub named_exports: Vec<SymbolId>,
  pub default_export: Option<Entity<'a>>,
  pub scope_context: ScopeContext<'a>,
  pub pending_deps: FxHashSet<ExhaustiveCallback<'a>>,
  pub builtins: Builtins<'a>,

  pub debug: usize,
}

impl<'a> Analyzer<'a> {
  pub fn new(tree_shaker: TreeShaker<'a>, semantic: Semantic<'a>) -> Self {
    let config = tree_shaker.0.config;
    let allocator = tree_shaker.0.allocator;
    let factory = tree_shaker.0.factory;

    Analyzer {
      tree_shaker,
      config,
      allocator,
      factory,
      line_index: LineIndex::new(semantic.source_text()),
      semantic,
      span_stack: vec![],
      data: Default::default(),
      referred_deps: Default::default(),
      conditional_data: Default::default(),
      loop_data: Default::default(),
      mangler: Mangler::new(config.mangling, allocator),
      named_exports: Vec::new(),
      default_export: None,
      scope_context: ScopeContext::new(factory),
      pending_deps: Default::default(),
      builtins: Builtins::new(config, factory),
      debug: 0,
    }
  }

  pub fn exec_program(&mut self, node: &'a Program<'a>) {
    // Top level is always preserved
    let top_level_call_id = self.call_scope().call_id;
    self.refer_dep(top_level_call_id);

    let data = self.load_data::<StatementVecData>(AstKind2::Program(node));
    self.exec_statement_vec(data, &node.body);

    self.consume_exports();

    let mut round = 0usize;
    loop {
      round += 1;
      if round > 1000 {
        panic!("Possible infinite loop in post analysis");
      }

      let mut dirty = false;
      dirty |= self.consume_top_level_uncaught();
      dirty |= self.call_exhaustive_callbacks();
      dirty |= self.post_analyze_handle_conditional();
      dirty |= self.post_analyze_handle_loops();
      if !dirty {
        break;
      }
    }

    self.scope_context.assert_final_state();

    // println!("debug: {:?}", self.debug);

    #[cfg(feature = "flame")]
    flamescope::dump(&mut std::fs::File::create("flamescope.json").unwrap()).unwrap();
  }

  pub fn consume_exports(&mut self) {
    if let Some(entity) = self.default_export.take() {
      entity.consume(self)
    }
    for symbol in self.named_exports.clone() {
      let entity = self.read_symbol(symbol).unwrap();
      entity.consume(self);
    }
  }

  pub fn consume_top_level_uncaught(&mut self) -> bool {
    let thrown_values = &mut self.call_scope_mut().try_scopes.last_mut().unwrap().thrown_values;
    if thrown_values.is_empty() {
      false
    } else {
      let values = mem::take(thrown_values);
      self.consume(values);
      true
    }
  }
}

impl<'a> Analyzer<'a> {
  pub fn set_data(&mut self, key: impl Into<DepId>, data: impl Default + 'a) {
    self.data.insert(key.into(), unsafe { mem::transmute(Box::new(data)) });
  }

  pub fn get_data_or_insert_with<D: 'a>(
    &mut self,
    key: impl Into<DepId>,
    default: impl FnOnce() -> D,
  ) -> &'a mut D {
    let boxed =
      self.data.entry(key.into()).or_insert_with(|| unsafe { mem::transmute(Box::new(default())) });
    unsafe { mem::transmute(boxed.as_mut()) }
  }

  pub fn load_data<D: Default + 'a>(&mut self, key: impl Into<DepId>) -> &'a mut D {
    self.get_data_or_insert_with(key, Default::default)
  }

  pub fn current_span(&self) -> Span {
    *self.span_stack.last().unwrap()
  }

  pub fn add_diagnostic(&mut self, message: impl Into<String>) {
    let span = self.current_span();
    let start = self.line_index.line_col(span.start.into());
    let end = self.line_index.line_col(span.end.into());
    let span_text =
      format!(" at {}:{}-{}:{}", start.line + 1, start.col + 1, end.line + 1, end.col + 1);
    self.tree_shaker.0.diagnostics.borrow_mut().insert(message.into() + &span_text);
  }

  pub fn push_span(&mut self, node: &'a impl GetSpan) {
    self.span_stack.push(node.span());
  }

  pub fn pop_span(&mut self) {
    self.span_stack.pop();
  }
}

impl<'a> From<Analyzer<'a>> for &'a Allocator {
  fn from(val: Analyzer<'a>) -> Self {
    val.allocator
  }
}
