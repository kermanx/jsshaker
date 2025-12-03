use std::mem;

use oxc::allocator;

use super::variable_scope::VariableScopeId;
use crate::{
  analyzer::Analyzer,
  dep::{DepAtom, DepTrait},
  entity::Entity,
  utils::CalleeInfo,
  utils::flame,
};

pub struct CallScope<'a> {
  pub call_id: DepAtom,
  pub callee: CalleeInfo<'a>,
  pub old_variable_scope_stack: Vec<VariableScopeId>,
  pub cf_scope_depth: usize,
  pub body_variable_scope: VariableScopeId,
  pub returned_values: Vec<Entity<'a>>,
  pub is_async: bool,
  pub is_generator: bool,
  pub need_consume_arguments: bool,

  #[cfg(feature = "flame")]
  pub scope_guard: flame::SpanGuard,
}

impl<'a> CallScope<'a> {
  pub fn new_in(
    call_id: DepAtom,
    callee: CalleeInfo<'a>,
    old_variable_scope_stack: Vec<VariableScopeId>,
    cf_scope_depth: usize,
    body_variable_scope: VariableScopeId,
    is_async: bool,
    is_generator: bool,
  ) -> Self {
    CallScope {
      call_id,
      callee,
      old_variable_scope_stack,
      cf_scope_depth,
      body_variable_scope,
      returned_values: Vec::new(),
      is_async,
      is_generator,
      need_consume_arguments: false,

      #[cfg(feature = "flame")]
      scope_guard: flame::start_guard(callee.debug_name.to_string()),
    }
  }

  pub fn finalize(self, analyzer: &mut Analyzer<'a>) -> (Vec<VariableScopeId>, Entity<'a>) {
    let value = match &self.returned_values[..] {
      [] => analyzer.factory.never,
      [v] => *v,
      [v1, v2] => analyzer.factory.union((*v1, *v2)),
      values => analyzer
        .factory
        .union(allocator::Vec::from_iter_in(values.iter().copied(), analyzer.allocator)),
    };

    #[cfg(feature = "flame")]
    self.scope_guard.end();

    (self.old_variable_scope_stack, value)
  }
}

impl<'a> Analyzer<'a> {
  pub fn return_value(&mut self, value: Entity<'a>, dep: impl DepTrait<'a> + 'a) {
    let call_scope = self.call_scope();
    let exec_dep = self.get_exec_dep(call_scope.cf_scope_depth);
    let value = self.factory.computed(value, (exec_dep, dep));

    let call_scope = self.call_scope_mut();
    call_scope.returned_values.push(value);

    let target_depth = call_scope.cf_scope_depth;
    self.exit_to(target_depth);
  }

  pub fn consume_arguments(&mut self) -> bool {
    let scope = self.call_scope().body_variable_scope;
    self.consume_arguments_on_scope(scope)
  }

  pub fn consume_return_values(&mut self) {
    let call_scope = self.call_scope_mut();
    let values = mem::take(&mut call_scope.returned_values);
    for value in values {
      self.consume(value);
    }
  }
}
