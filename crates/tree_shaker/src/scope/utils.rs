use crate::{analyzer::Analyzer, consumable::ConsumableVec};
use oxc::semantic::SymbolId;
use std::mem;

use super::cf_scope::CfScopeId;

impl<'a> Analyzer<'a> {
  pub fn find_first_different_cf_scope(&self, another: CfScopeId) -> usize {
    self.scope_context.cf.find_lca(another).0 + 1
  }

  /// Returns (has_exhaustive, indeterminate, exec_deps)
  pub fn pre_mutate_object(
    &mut self,
    cf_scope: CfScopeId,
    object_id: SymbolId,
  ) -> (bool, bool, ConsumableVec<'a>) {
    let target_depth = self.find_first_different_cf_scope(cf_scope);

    let mut has_exhaustive = false;
    let mut indeterminate = false;
    let mut exec_deps = vec![];
    for depth in target_depth..self.scope_context.cf.stack.len() {
      let scope = self.scope_context.cf.get_mut_from_depth(depth);
      if !has_exhaustive {
        has_exhaustive |=
          scope.mark_exhaustive_write((self.scope_context.object_scope_id, object_id));
      }
      indeterminate |= scope.is_indeterminate();
      if let Some(dep) = scope.deps.try_collect(self.factory) {
        exec_deps.push(dep);
      }
    }

    self.add_exhaustive_callbacks(true, (self.scope_context.object_scope_id, object_id));

    (has_exhaustive, indeterminate, exec_deps)
  }

  pub fn mark_object_property_exhaustive_read(&mut self, cf_scope: CfScopeId, object_id: SymbolId) {
    let target_depth = self.find_first_different_cf_scope(cf_scope);
    self.mark_exhaustive_read((self.scope_context.object_scope_id, object_id), target_depth);
  }

  pub fn mark_object_consumed(&mut self, cf_scope: CfScopeId, object_id: SymbolId) {
    let target_depth = self.find_first_different_cf_scope(cf_scope);
    let mut marked = false;
    for depth in target_depth..self.scope_context.cf.stack.len() {
      let scope = self.scope_context.cf.get_mut_from_depth(depth);
      if !marked {
        marked = scope.mark_exhaustive_write((self.scope_context.object_scope_id, object_id));
      }
      mem::take(&mut scope.deps).consume_all(self);
    }
    self.add_exhaustive_callbacks(true, (self.scope_context.object_scope_id, object_id));
  }
}
