pub mod call_scope;
pub mod cf_scope;
// pub mod r#loop;
mod scope_tree;
// mod utils;
pub mod rw_tracking;
pub mod variable_scope;

use call_scope::CallScope;
use cf_scope::CfScope;
pub use cf_scope::{CfScopeId, CfScopeKind};
use scope_tree::ScopeTree;
use variable_scope::VariableScope;
pub use variable_scope::VariableScopeId;

use crate::{
  analyzer::{Analyzer, Factory},
  dep::{Dep, DepAtom, DepTrait, DepVec},
  entity::Entity,
  module::ModuleId,
  utils::{CalleeInfo, CalleeNode},
  value::{ObjectId, cache::FnCacheTrackingData},
};

pub struct Scoping<'a> {
  pub call: Vec<CallScope<'a>>,
  pub variable: ScopeTree<VariableScopeId, VariableScope<'a>>,
  pub cf: ScopeTree<CfScopeId, CfScope<'a>>,
  pub pure: usize,
  pub try_catch_depth: Option<usize>,

  pub object_symbol_counter: usize,
}

impl<'a> Scoping<'a> {
  pub fn new(factory: &Factory<'a>) -> Self {
    let mut variable = ScopeTree::new();
    variable.push(VariableScope::new_with_this(factory.unknown));
    let mut cf = ScopeTree::new();
    cf.push(CfScope::new(CfScopeKind::Root, factory.vec(), false));
    Scoping {
      call: vec![CallScope::new_in(
        DepAtom::from_counter(),
        CalleeInfo {
          module_id: ModuleId::from(0),
          node: CalleeNode::Root,
          instance_id: factory.alloc_instance_id(),
          #[cfg(feature = "flame")]
          debug_name: "<Root>",
        },
        vec![],
        0,
        VariableScopeId::from(0),
        false,
        false,
      )],
      variable,
      cf,
      pure: 0,
      try_catch_depth: None,

      object_symbol_counter: 128,
    }
  }

  pub fn alloc_object_id(&mut self) -> ObjectId {
    self.object_symbol_counter += 1;
    ObjectId::from_usize(self.object_symbol_counter)
  }
}

impl<'a> Analyzer<'a> {
  pub fn call_scope(&self) -> &CallScope<'a> {
    self.scoping.call.last().unwrap()
  }

  pub fn call_scope_mut(&mut self) -> &mut CallScope<'a> {
    self.scoping.call.last_mut().unwrap()
  }

  pub fn cf_scope(&self) -> &CfScope<'a> {
    self.scoping.cf.get_current()
  }

  pub fn cf_scope_mut(&mut self) -> &mut CfScope<'a> {
    self.scoping.cf.get_current_mut()
  }

  pub fn cf_scope_id_of_call_scope(&self) -> CfScopeId {
    let depth = self.call_scope().cf_scope_depth;
    self.scoping.cf.stack[depth]
  }

  pub fn variable_scope(&self) -> &VariableScope<'a> {
    self.scoping.variable.get_current()
  }

  pub fn variable_scope_mut(&mut self) -> &mut VariableScope<'a> {
    self.scoping.variable.get_current_mut()
  }

  pub fn is_inside_pure(&self) -> bool {
    // TODO: self.scoping.pure > 0
    false
  }

  pub fn replace_variable_scope_stack(
    &mut self,
    new_stack: Vec<VariableScopeId>,
  ) -> Vec<VariableScopeId> {
    self.scoping.variable.replace_stack(new_stack)
  }

  pub fn push_call_scope(
    &mut self,
    callee: CalleeInfo<'a>,
    call_dep: Dep<'a>,
    variable_scope_stack: Vec<VariableScopeId>,
    is_async: bool,
    is_generator: bool,
    consume: bool,
  ) {
    let dep_id = DepAtom::from_counter();
    if consume {
      self.refer_dep(dep_id);
    }

    self.module_stack.push(callee.module_id);
    let old_variable_scope_stack = self.replace_variable_scope_stack(variable_scope_stack);
    let body_variable_scope = self.push_variable_scope();
    let cf_scope_depth = self.push_cf_scope_with_deps(
      CfScopeKind::Function(self.allocator.alloc(FnCacheTrackingData { has_outer_deps: false })),
      self.factory.vec1(self.dep((call_dep, dep_id))),
      false,
    );

    self.scoping.call.push(CallScope::new_in(
      dep_id,
      callee,
      old_variable_scope_stack,
      cf_scope_depth,
      body_variable_scope,
      is_async,
      is_generator,
    ));
  }

  pub fn pop_call_scope(&mut self) -> (Entity<'a>, FnCacheTrackingData) {
    let scope = self.scoping.call.pop().unwrap();
    let (old_variable_scope_stack, ret_val) = scope.finalize(self);
    let cf_scope_id = self.pop_cf_scope();
    let cf_scope = self.scoping.cf.get(cf_scope_id);
    let CfScopeKind::Function(tracking_data) = &cf_scope.kind else {
      unreachable!();
    };
    let tracking_data = **tracking_data;

    self.pop_variable_scope();
    self.replace_variable_scope_stack(old_variable_scope_stack);
    self.module_stack.pop();
    (ret_val, tracking_data)
  }

  pub fn push_variable_scope(&mut self) -> VariableScopeId {
    self.scoping.variable.push(VariableScope::new())
  }

  pub fn pop_variable_scope(&mut self) -> VariableScopeId {
    self.scoping.variable.pop()
  }

  pub fn push_cf_scope(&mut self, kind: CfScopeKind<'a>, indeterminate: bool) -> usize {
    self.push_cf_scope_with_deps(kind, self.factory.vec(), indeterminate)
  }

  pub fn push_cf_scope_with_deps(
    &mut self,
    kind: CfScopeKind<'a>,
    deps: DepVec<'a>,
    indeterminate: bool,
  ) -> usize {
    self.scoping.cf.push(CfScope::new(kind, deps, indeterminate));
    self.scoping.cf.current_depth()
  }

  pub fn push_indeterminate_cf_scope(&mut self) {
    self.push_cf_scope(CfScopeKind::Indeterminate, true);
  }

  pub fn push_dependent_cf_scope(&mut self, dep: impl DepTrait<'a> + 'a) {
    self.push_cf_scope_with_deps(
      CfScopeKind::Dependent,
      self.factory.vec1(dep.uniform(self.allocator)),
      false,
    );
  }

  pub fn pop_cf_scope(&mut self) -> CfScopeId {
    self.scoping.cf.pop()
  }

  pub fn pop_multiple_cf_scopes(&mut self, count: usize) -> Option<Dep<'a>> {
    let mut exec_deps = self.factory.vec();
    for _ in 0..count {
      let id = self.scoping.cf.stack.pop().unwrap();
      if let Some(dep) = self.scoping.cf.get_mut(id).deps.try_collect(self.factory) {
        exec_deps.push(dep);
      }
    }
    if exec_deps.is_empty() { None } else { Some(self.dep(exec_deps)) }
  }

  pub fn pop_cf_scope_and_get_mut(&mut self) -> &mut CfScope<'a> {
    let id = self.pop_cf_scope();
    self.scoping.cf.get_mut(id)
  }
}
