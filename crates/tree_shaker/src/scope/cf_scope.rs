use crate::{
  analyzer::Analyzer,
  consumable::{Consumable, ConsumableCollector, ConsumableVec},
  utils::ast::AstKind2,
};
use oxc::{ast::ast::LabeledStatement, semantic::SymbolId, span::Atom};
use oxc_index::define_index_type;
use rustc_hash::FxHashSet;
use std::mem;

use super::variable_scope::VariableScopeId;

define_index_type! {
  pub struct CfScopeId = u32;
}

#[derive(Debug, Default)]
pub struct ExhaustiveData {
  pub clean: bool,
  pub deps: FxHashSet<(VariableScopeId, SymbolId)>,
}

#[derive(Debug)]
pub enum CfScopeKind<'a> {
  Root,
  Module,
  Labeled(&'a LabeledStatement<'a>),
  Function,
  Loop,
  Switch,

  Dependent,
  Indeterminate,
  Exhaustive(ExhaustiveData),
  ExitBlocker(Option<usize>),
}

impl<'a> CfScopeKind<'a> {
  pub fn is_function(&self) -> bool {
    matches!(self, CfScopeKind::Function)
  }

  pub fn is_breakable_without_label(&self) -> bool {
    matches!(self, CfScopeKind::Loop | CfScopeKind::Switch)
  }

  pub fn is_continuable(&self) -> bool {
    matches!(self, CfScopeKind::Loop)
  }

  pub fn matches_label(&self, label: &'a Atom<'a>) -> Option<&'a LabeledStatement<'a>> {
    match self {
      CfScopeKind::Labeled(stmt) if stmt.label.name == label => Some(stmt),
      _ => None,
    }
  }

  pub fn is_exhaustive(&self) -> bool {
    matches!(self, CfScopeKind::Exhaustive(_))
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferredState {
  Never,
  ReferredClean,
  ReferredDirty,
}

#[derive(Debug)]
pub struct CfScope<'a> {
  pub kind: CfScopeKind<'a>,
  pub deps: ConsumableCollector<'a>,
  pub referred_state: ReferredState,
  pub exited: Option<bool>,
}

impl<'a> CfScope<'a> {
  pub fn new(kind: CfScopeKind<'a>, deps: ConsumableVec<'a>, exited: Option<bool>) -> Self {
    CfScope {
      kind,
      deps: ConsumableCollector::new(deps),
      referred_state: ReferredState::Never,
      exited,
    }
  }

  pub fn push_dep(&mut self, dep: Consumable<'a>) {
    self.deps.push(dep);
    if self.referred_state == ReferredState::ReferredClean {
      self.referred_state = ReferredState::ReferredDirty;
    }
  }

  pub fn update_exited(&mut self, exited: Option<bool>, dep: Option<Consumable<'a>>) {
    if self.exited != Some(true) {
      self.exited = exited;
      if let Some(dep) = dep {
        self.push_dep(dep);
      }
    }
  }

  pub fn must_exited(&self) -> bool {
    matches!(self.exited, Some(true))
  }

  pub fn is_indeterminate(&self) -> bool {
    self.exited.is_none()
  }

  pub fn exhaustive_data_mut(&mut self) -> Option<&mut ExhaustiveData> {
    match &mut self.kind {
      CfScopeKind::Exhaustive(data) => Some(data),
      _ => None,
    }
  }

  pub fn mark_exhaustive_read(&mut self, variable: (VariableScopeId, SymbolId)) {
    if let Some(data) = self.exhaustive_data_mut() {
      if data.clean {
        data.deps.insert(variable);
      }
    }
  }

  pub fn mark_exhaustive_write(&mut self, variable: (VariableScopeId, SymbolId)) -> bool {
    if let Some(data) = self.exhaustive_data_mut() {
      if data.clean && data.deps.contains(&variable) {
        data.clean = false;
      }
      true
    } else {
      false
    }
  }

  pub fn iterate_exhaustively(&mut self) -> bool {
    let exited = self.must_exited();
    let data = self.exhaustive_data_mut().unwrap();
    let clean = data.clean;
    data.clean = true;
    if !clean && !exited {
      data.deps.clear();
      true
    } else {
      false
    }
  }
}

impl<'a> Analyzer<'a> {
  pub fn exec_indeterminately<T>(&mut self, runner: impl FnOnce(&mut Analyzer<'a>) -> T) -> T {
    self.push_indeterminate_cf_scope();
    let result = runner(self);
    self.pop_cf_scope();
    result
  }

  pub fn get_exec_dep(&mut self, target_depth: usize) -> Consumable<'a> {
    let mut deps = vec![];
    for id in target_depth..self.scope_context.cf.stack.len() {
      let scope = self.scope_context.cf.get_mut_from_depth(id);
      if let Some(dep) = scope.deps.try_collect(self.factory) {
        deps.push(dep);
      }
    }
    self.consumable(deps)
  }

  pub fn exit_to(&mut self, target_depth: usize) {
    self.exit_to_impl(target_depth, self.scope_context.cf.stack.len(), true, None);
  }

  pub fn exit_to_not_must(&mut self, target_depth: usize) {
    self.exit_to_impl(target_depth, self.scope_context.cf.stack.len(), false, None);
  }

  /// `None` => Interrupted by if branch
  /// `Some` => Accumulated dependencies, may be `None`
  pub fn exit_to_impl(
    &mut self,
    target_depth: usize,
    from_depth: usize,
    mut must_exit: bool,
    mut acc_dep: Option<Consumable<'a>>,
  ) -> Option<Option<Consumable<'a>>> {
    for depth in (target_depth..from_depth).rev() {
      let id = self.scope_context.cf.stack[depth];
      let cf_scope = self.scope_context.cf.get_mut(id);
      let this_dep = cf_scope.deps.try_collect(self.factory);

      // Update exited state
      if must_exit {
        let is_indeterminate = cf_scope.is_indeterminate();
        cf_scope.update_exited(Some(true), acc_dep);

        // Stop exiting outer scopes if one inner scope is indeterminate.
        if is_indeterminate {
          must_exit = false;
          if let CfScopeKind::ExitBlocker(target) = &mut cf_scope.kind {
            // For the `if` statement, do not mark the outer scopes as indeterminate here.
            // Instead, let the `if` statement handle it.
            assert!(target.is_none());
            *target = Some(target_depth);
            return None;
          }
        }
      } else {
        cf_scope.update_exited(None, acc_dep);
      }

      // Accumulate the dependencies
      if let Some(this_dep) = this_dep {
        acc_dep = if let Some(acc_dep) = acc_dep {
          Some(self.consumable((this_dep, acc_dep)))
        } else {
          Some(this_dep)
        };
      }
    }
    Some(acc_dep)
  }

  /// If the label is used, `true` is returned.
  pub fn break_to_label(&mut self, label: Option<&'a Atom<'a>>) -> bool {
    let mut is_closest_breakable = true;
    let mut target_depth = None;
    let mut label_used = false;
    for (idx, cf_scope) in self.scope_context.cf.iter_stack().enumerate().rev() {
      if cf_scope.kind.is_function() {
        break;
      }
      let breakable_without_label = cf_scope.kind.is_breakable_without_label();
      if let Some(label) = label {
        if let Some(label) = cf_scope.kind.matches_label(label) {
          if !is_closest_breakable || !breakable_without_label {
            self.referred_deps.refer_dep(AstKind2::LabeledStatement(label));
            label_used = true;
          }
          target_depth = Some(idx);
          break;
        }
        if breakable_without_label {
          is_closest_breakable = false;
        }
      } else if breakable_without_label {
        target_depth = Some(idx);
        break;
      }
    }
    self.exit_to(target_depth.unwrap());
    label_used
  }

  /// If the label is used, `true` is returned.
  pub fn continue_to_label(&mut self, label: Option<&'a Atom<'a>>) -> bool {
    let mut is_closest_continuable = true;
    let mut target_depth = None;
    let mut label_used = false;
    for (idx, cf_scope) in self.scope_context.cf.iter_stack().enumerate().rev() {
      if cf_scope.kind.is_function() {
        break;
      }
      if let Some(label) = label {
        if let Some(label) = cf_scope.kind.matches_label(label) {
          if !is_closest_continuable {
            self.referred_deps.refer_dep(AstKind2::LabeledStatement(label));
            label_used = true;
          }
          target_depth = Some(idx);
          break;
        }
        is_closest_continuable = false;
      } else if cf_scope.kind.is_continuable() {
        target_depth = Some(idx);
        break;
      }
    }
    if target_depth.is_none() {
      panic!("label: {:?}, is_closest_continuable: {}", label, is_closest_continuable);
    }
    self.exit_to(target_depth.unwrap());
    label_used
  }

  pub fn refer_to_global(&mut self) {
    if self.is_inside_pure() {
      return;
    }

    for depth in (0..self.scope_context.cf.stack.len()).rev() {
      let scope = self.scope_context.cf.get_mut_from_depth(depth);
      match scope.referred_state {
        ReferredState::Never => {
          scope.referred_state = ReferredState::ReferredClean;
          mem::take(&mut scope.deps).consume_all(self);
        }
        ReferredState::ReferredClean => break,
        ReferredState::ReferredDirty => {
          scope.referred_state = ReferredState::ReferredClean;
          mem::take(&mut scope.deps).consume_all(self);
          for depth in (0..depth).rev() {
            let scope = self.scope_context.cf.get_mut_from_depth(depth);
            match scope.referred_state {
              ReferredState::Never => unreachable!("Logic error in refer_to_global"),
              ReferredState::ReferredClean => break,
              ReferredState::ReferredDirty => {
                scope.deps.force_clear();
                scope.referred_state = ReferredState::ReferredClean;
              }
            }
          }
          break;
        }
      }
    }

    self.call_exhaustive_callbacks();
  }
}
