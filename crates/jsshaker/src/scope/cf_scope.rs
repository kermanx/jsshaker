use oxc::{ast::ast::LabeledStatement, span::Atom};

use crate::{
  analyzer::{Analyzer, exhaustive::ExhaustiveData},
  define_box_bump_idx,
  dep::{Dep, DepCollector, DepVec},
  utils::ast::AstKind2,
  value::cache::FnCacheTrackingData,
};

define_box_bump_idx! {
  pub struct CfScopeId;
}

#[derive(Debug)]
pub enum CfScopeKind<'a> {
  Root,
  Module,
  Labeled(&'a LabeledStatement<'a>),
  Function(&'a mut FnCacheTrackingData<'a>),
  LoopBreak,
  LoopContinue,
  Switch,

  Dependent,
  Indeterminate,
  Exhaustive(&'a mut ExhaustiveData<'a>),
  ExitBlocker(Option<usize>),
}

impl<'a> CfScopeKind<'a> {
  pub fn is_function(&self) -> bool {
    matches!(self, CfScopeKind::Function(_))
  }

  pub fn is_breakable_without_label(&self) -> bool {
    matches!(self, CfScopeKind::LoopBreak | CfScopeKind::Switch)
  }

  pub fn is_continuable(&self) -> bool {
    matches!(self, CfScopeKind::LoopContinue)
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
pub enum DeoptimizeState {
  Never,
  DeoptimizedClean,
  DeoptimizedDirty,
}

#[derive(Debug)]
pub struct CfScope<'a> {
  pub kind: CfScopeKind<'a>,
  pub deps: DepCollector<'a>,
  pub deoptimize_state: DeoptimizeState,
  pub exited: Option<bool>,
}

impl<'a> CfScope<'a> {
  pub fn new(kind: CfScopeKind<'a>, deps: DepVec<'a>, indeterminate: bool) -> Self {
    CfScope {
      kind,
      deps: DepCollector::new(deps),
      deoptimize_state: DeoptimizeState::Never,
      exited: if indeterminate { None } else { Some(false) },
    }
  }

  pub fn push_dep(&mut self, dep: Dep<'a>) {
    self.deps.push(dep);
    if self.deoptimize_state == DeoptimizeState::DeoptimizedClean {
      self.deoptimize_state = DeoptimizeState::DeoptimizedDirty;
    }
  }

  pub fn update_exited(&mut self, exited: Option<bool>, dep: Option<Dep<'a>>) {
    if self.exited != Some(true) {
      self.exited = exited;
      if let Some(dep) = dep {
        self.push_dep(dep);
      }
    }
  }

  pub fn reset_indeterminate(&mut self) {
    self.exited = None;
    self.deps.force_clear();
  }

  pub fn must_exited(&self) -> bool {
    matches!(self.exited, Some(true))
  }

  pub fn is_indeterminate(&self) -> bool {
    self.exited.is_none()
  }

  pub fn is_exhaustive(&self) -> bool {
    matches!(self.kind, CfScopeKind::Exhaustive(_))
  }

  pub fn exhaustive_data_mut(&mut self) -> Option<&mut ExhaustiveData<'a>> {
    match &mut self.kind {
      CfScopeKind::Exhaustive(data) => Some(data),
      _ => None,
    }
  }

  pub fn fn_cache_tracking_data_mut(&mut self) -> Option<&mut FnCacheTrackingData<'a>> {
    match &mut self.kind {
      CfScopeKind::Function(data) => Some(data),
      _ => None,
    }
  }

  pub fn post_exhaustive_iterate(&mut self) -> bool {
    let exited = self.must_exited();
    let data = self.exhaustive_data_mut().unwrap();
    if !data.clean && !exited {
      if let Some(temp_deps) = &mut data.temp_deps {
        temp_deps.clear();
        data.clean = true;
        true
      } else {
        false
      }
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

  pub fn find_first_different_cf_scope(&self, another: CfScopeId) -> usize {
    self.scoping.cf.find_lca(another).0 + 1
  }

  pub fn get_exec_dep(&mut self, target_depth: usize) -> (DepVec<'a>, bool) {
    let mut deps = self.factory.vec();
    let mut indeterminate = false;
    for id in target_depth..self.scoping.cf.stack.len() {
      let scope = self.scoping.cf.get_mut_from_depth(id);
      if let Some(dep) = scope.deps.collect(self.factory) {
        deps.push(dep);
      }
      indeterminate |= scope.is_indeterminate();
    }
    (deps, indeterminate)
  }

  pub fn exit_to(&mut self, target_depth: usize) {
    self.exit_to_impl(target_depth, self.scoping.cf.stack.len(), true, None);
  }

  pub fn exit_to_not_must(&mut self, target_depth: usize) {
    self.exit_to_impl(target_depth, self.scoping.cf.stack.len(), false, None);
  }

  /// `None` => Interrupted by if branch
  /// `Some` => Accumulated dependencies, may be `None`
  pub fn exit_to_impl(
    &mut self,
    target_depth: usize,
    from_depth: usize,
    mut must_exit: bool,
    mut acc_dep: Option<Dep<'a>>,
  ) -> Option<Option<Dep<'a>>> {
    for depth in (target_depth..from_depth).rev() {
      let id = self.scoping.cf.stack[depth];
      let cf_scope = self.scoping.cf.get_mut(id);

      if cf_scope.must_exited() {
        return Some(Some(self.factory.no_dep));
      }

      let this_dep = cf_scope.deps.collect(self.factory);

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
          Some(self.dep((this_dep, acc_dep)))
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
    for (idx, cf_scope) in self.scoping.cf.iter_stack().enumerate().rev() {
      if cf_scope.kind.is_function() {
        break;
      }
      let breakable_without_label = cf_scope.kind.is_breakable_without_label();
      if let Some(label) = label {
        if let Some(label) = cf_scope.kind.matches_label(label) {
          if !is_closest_breakable || !breakable_without_label {
            self.deoptimized_atoms.deoptimize_atom(AstKind2::LabeledStatement(label));
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
    for (idx, cf_scope) in self.scoping.cf.iter_stack().enumerate().rev() {
      if cf_scope.kind.is_function() {
        break;
      }
      if let Some(label) = label {
        if let Some(label) = cf_scope.kind.matches_label(label) {
          if !is_closest_continuable {
            self.deoptimized_atoms.deoptimize_atom(AstKind2::LabeledStatement(label));
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

  pub fn exit_by_throw(&mut self, explicit: bool) -> usize {
    let target_depth = self.scoping.try_catch_depth.unwrap_or_else(|| {
      if explicit {
        self.global_effect();
        return 0;
      }
      let mut target_depth = 0;
      for (idx, cf_scope) in self.scoping.cf.iter_stack().enumerate().rev() {
        if cf_scope.exited != Some(false) {
          target_depth = idx;
          break;
        }
      }
      target_depth
    });
    self.exit_to(target_depth);
    target_depth
  }

  pub fn global_effect(&mut self) {
    let mut deps = vec![];
    for depth in (0..self.scoping.cf.stack.len()).rev() {
      let scope = self.scoping.cf.get_mut_from_depth(depth);
      match scope.deoptimize_state {
        DeoptimizeState::Never => {
          scope.deoptimize_state = DeoptimizeState::DeoptimizedClean;
          deps.push(scope.deps.take(self.factory));
        }
        DeoptimizeState::DeoptimizedClean => break,
        DeoptimizeState::DeoptimizedDirty => {
          scope.deoptimize_state = DeoptimizeState::DeoptimizedClean;
          deps.push(scope.deps.take(self.factory));

          for depth in (0..depth).rev() {
            let scope = self.scoping.cf.get_mut_from_depth(depth);
            match scope.deoptimize_state {
              DeoptimizeState::Never => unreachable!("Logic error in global_effect"),
              DeoptimizeState::DeoptimizedClean => break,
              DeoptimizeState::DeoptimizedDirty => {
                scope.deps.force_clear();
                scope.deoptimize_state = DeoptimizeState::DeoptimizedClean;
              }
            }
          }
          break;
        }
      }
    }
    self.consume(deps);
    self.call_exhaustive_callbacks();
  }
}
