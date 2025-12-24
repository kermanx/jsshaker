pub mod call_scope;
pub mod cf_scope;
// pub mod r#loop;
mod stacked_tree;
// mod utils;
mod linked_tree;
pub mod variable_scope;

use std::mem;

use call_scope::CallScope;
use cf_scope::CfScope;
pub use cf_scope::{CfScopeId, CfScopeKind};
use stacked_tree::StackedTree;
use variable_scope::VariableScope;
pub use variable_scope::VariableScopeId;

use crate::{
  analyzer::{Analyzer, Factory},
  dep::{Dep, DepAtom, DepTrait, DepVec},
  entity::Entity,
  module::ModuleId,
  scope::linked_tree::LinkedTree,
  utils::{CalleeInfo, CalleeNode},
  value::{cache::FnCacheTrackingData, call::FnCallInfo},
};

pub struct Scoping<'a> {
  pub call: Vec<CallScope<'a>>,
  pub variable: LinkedTree<'a, VariableScopeId, VariableScope<'a>>,
  pub cf: StackedTree<'a, CfScopeId, CfScope<'a>>,
  pub root_cf_scope: CfScopeId,
  pub try_catch_depth: Option<usize>,

  pub object_symbol_counter: usize,
}

impl<'a> Scoping<'a> {
  pub fn new(factory: &mut Factory<'a>) -> Self {
    let mut variable = LinkedTree::new_in(factory.allocator);
    let root_variable_scope =
      variable.push(VariableScope::new_in_with_this(factory.allocator, factory.unknown));
    let mut cf = StackedTree::new_in(factory.allocator);
    let root_cf_scope = cf.push(CfScope::new(CfScopeKind::Root, factory.vec(), false));
    factory.root_cf_scope = Some(root_cf_scope);
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
        ModuleId::from(0),
        None,
        0,
        root_variable_scope,
        false,
        false,
      )],
      variable,
      cf,
      root_cf_scope,
      try_catch_depth: None,

      object_symbol_counter: 128,
    }
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

  pub fn variable_scope(&self) -> &VariableScope<'a> {
    self.scoping.variable.get_current()
  }

  pub fn variable_scope_mut(&mut self) -> &mut VariableScope<'a> {
    self.scoping.variable.get_current_mut()
  }

  pub fn replace_variable_scope(
    &mut self,
    new_top: Option<VariableScopeId>,
  ) -> Option<VariableScopeId> {
    self.scoping.variable.replace_top(new_top)
  }

  pub fn push_call_scope(&mut self, info: FnCallInfo<'a>, is_async: bool, is_generator: bool) {
    if info.consume {
      self.refer_dep(info.call_id);
    }

    let old_module = self.set_current_module(info.func.callee.module_id);
    let old_variable_scope_stack = self.replace_variable_scope(info.func.lexical_scope);
    let body_variable_scope = self.push_variable_scope();
    let cf_scope_depth = self.push_cf_scope_with_deps(
      CfScopeKind::Function(
        self.allocator.alloc(FnCacheTrackingData::new_in(self.allocator, info)),
      ),
      self.factory.vec1(info.call_dep),
      false,
    );

    self.scoping.call.push(CallScope::new_in(
      info.call_id,
      info.func.callee,
      old_module,
      old_variable_scope_stack,
      cf_scope_depth,
      body_variable_scope,
      is_async,
      is_generator,
    ));
  }

  pub fn pop_call_scope(&mut self) -> (Entity<'a>, FnCacheTrackingData<'a>) {
    let scope = self.scoping.call.pop().unwrap();
    let ret_val = scope.ret_val(self);
    let cf_scope_id = self.pop_cf_scope();
    let cf_scope = self.scoping.cf.get_mut(cf_scope_id);
    let CfScopeKind::Function(tracking_data) = &mut cf_scope.kind else {
      unreachable!();
    };
    let tracking_data = mem::take(*tracking_data);

    self.pop_variable_scope();
    self.replace_variable_scope(scope.old_variable_scope);
    self.set_current_module(scope.old_module);
    (ret_val, tracking_data)
  }

  pub fn push_variable_scope(&mut self) -> VariableScopeId {
    self.scoping.variable.push(VariableScope::new_in(self.allocator))
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
      if let Some(dep) = self.scoping.cf.get_mut(id).deps.collect(self.factory) {
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
