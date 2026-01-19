use oxc::{
  ast::ast::{LabelIdentifier, LabeledStatement},
  span::Atom,
};

use crate::{
  analyzer::{Analyzer, exhaustive::ExhaustiveData},
  define_stacked_tree_idx,
  dep::{Dep, DepCollector, DepVec},
  value::cache::FnCacheTrackingData,
};

define_stacked_tree_idx! {
  pub struct CfScopeId;
}

#[derive(Debug)]
pub enum CfScopeKind<'a> {
  Root,
  Module,
  Labeled(&'a LabeledStatement<'a>),
  Function(Box<FnCacheTrackingData<'a>>),
  LoopBreak,
  LoopContinue,
  Switch,

  Dependent,
  NonDet,
  Exhaustive(Box<ExhaustiveData<'a>>),
  ExitBlocker(Option<usize>),
}

impl<'a> CfScopeKind<'a> {
  pub fn is_function(&self) -> bool {
    matches!(self, CfScopeKind::Function(_))
  }

  pub fn is_breakable(&self) -> bool {
    matches!(self, CfScopeKind::LoopBreak | CfScopeKind::Switch)
  }

  pub fn is_continuable(&self) -> bool {
    matches!(self, CfScopeKind::LoopContinue)
  }

  pub fn matches_label(&self, label: &'a Atom<'a>) -> bool {
    matches!(self, CfScopeKind::Labeled(stmt) if stmt.label.name == label)
  }

  pub fn is_exhaustive(&self) -> bool {
    matches!(self, CfScopeKind::Exhaustive(_))
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncludeState {
  Never,
  IncludedClean,
  IncludedDirty,
}

#[derive(Debug)]
pub struct CfScope<'a> {
  pub kind: CfScopeKind<'a>,
  pub deps: DepCollector<'a>,
  pub include_state: IncludeState,
  pub exited: Option<bool>,
}

impl<'a> CfScope<'a> {
  pub fn new(kind: CfScopeKind<'a>, deps: DepVec<'a>, non_det: bool) -> Self {
    CfScope {
      kind,
      deps: DepCollector::new(deps),
      include_state: IncludeState::Never,
      exited: if non_det { None } else { Some(false) },
    }
  }

  pub fn push_dep(&mut self, dep: Dep<'a>) {
    self.deps.push(dep);
    if self.include_state == IncludeState::IncludedClean {
      self.include_state = IncludeState::IncludedDirty;
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

  pub fn reset_non_det(&mut self) {
    self.exited = None;
    self.deps.force_clear();
  }

  pub fn must_exited(&self) -> bool {
    matches!(self.exited, Some(true))
  }

  pub fn non_det(&self) -> bool {
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
  pub fn exec_non_det<T>(&mut self, runner: impl FnOnce(&mut Analyzer<'a>) -> T) -> T {
    self.push_non_det_cf_scope();
    let result = runner(self);
    self.pop_cf_scope();
    result
  }

  pub fn find_first_different_cf_scope(&self, another: CfScopeId) -> usize {
    self.scoping.cf.find_lca(another).0 + 1
  }

  pub fn get_exec_dep(&mut self, target_depth: usize) -> (DepVec<'a>, bool) {
    let mut deps = self.factory.vec();
    let mut non_det = false;
    for id in target_depth..self.scoping.cf.stack_len() {
      let scope = self.scoping.cf.data_at_mut(id);
      if let Some(dep) = scope.deps.collect(self.factory) {
        deps.push(dep);
      }
      non_det |= scope.non_det();
    }
    (deps, non_det)
  }

  pub fn exit_to(&mut self, target_depth: usize) {
    self.exit_to_impl(target_depth, self.scoping.cf.stack_len(), true, None);
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
      let cf_scope = self.scoping.cf.data_at_mut(depth);

      if cf_scope.must_exited() {
        return Some(Some(self.factory.no_dep));
      }

      let this_dep = cf_scope.deps.collect(self.factory);

      // Update exited state
      if must_exit {
        let non_det = cf_scope.non_det();
        cf_scope.update_exited(Some(true), acc_dep);

        // Stop exiting outer scopes if one inner scope is non_det.
        if non_det {
          must_exit = false;
          if let CfScopeKind::ExitBlocker(target) = &mut cf_scope.kind {
            // For the `if` statement, do not mark the outer scopes as non_det here.
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

  pub fn break_to_label(&mut self, label: Option<&'a LabelIdentifier<'a>>) {
    let mut target_depth = None;
    for (depth, cf_scope) in self.scoping.cf.iter_stack().enumerate().rev() {
      if cf_scope.kind.is_function() {
        break;
      }
      if let Some(label) = label {
        if cf_scope.kind.matches_label(&label.name) {
          target_depth = Some(depth);
          break;
        }
      } else if cf_scope.kind.is_breakable() {
        target_depth = Some(depth);
        break;
      }
    }
    self.exit_to(target_depth.expect("No valid break target found"));
  }

  pub fn continue_to_label(&mut self, label: Option<&'a LabelIdentifier<'a>>) {
    let mut target_depth = None;
    for (depth, cf_scope) in self.scoping.cf.iter_stack().enumerate().rev() {
      if cf_scope.kind.is_function() {
        break;
      }
      if let Some(label) = label {
        if cf_scope.kind.matches_label(&label.name) {
          let mut continue_depth = depth + 1;
          while !self.scoping.cf.data_at(continue_depth).kind.is_continuable() {
            continue_depth += 1;
          }
          target_depth = Some(continue_depth);
          break;
        }
      } else if cf_scope.kind.is_continuable() {
        target_depth = Some(depth);
        break;
      }
    }
    self.exit_to(target_depth.expect("No valid continue target found"));
  }

  pub fn exit_by_throw(&mut self, explicit: bool) -> usize {
    let target_depth = self.scoping.try_catch_depth.unwrap_or_else(|| {
      if explicit {
        self.global_effect();
        return 0;
      }
      let mut target_depth = 0;
      for (depth, cf_scope) in self.scoping.cf.iter_stack().enumerate().rev() {
        if cf_scope.exited != Some(false) {
          target_depth = depth;
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
    let mut first_stage = true;
    for scope in self.scoping.cf.iter_stack_mut().rev() {
      if first_stage {
        match scope.include_state {
          IncludeState::Never => {
            scope.include_state = IncludeState::IncludedClean;
            if let Some(dep) = scope.deps.take(self.factory) {
              deps.push(dep);
            }
          }
          IncludeState::IncludedClean => break,
          IncludeState::IncludedDirty => {
            scope.include_state = IncludeState::IncludedClean;
            if let Some(dep) = scope.deps.take(self.factory) {
              deps.push(dep);
            }
            first_stage = false;
          }
        }
      } else {
        match scope.include_state {
          IncludeState::Never => unreachable!("Logic error in global_effect"),
          IncludeState::IncludedClean => break,
          IncludeState::IncludedDirty => {
            scope.deps.force_clear();
            scope.include_state = IncludeState::IncludedClean;
          }
        }
      }
    }
    self.consume(deps);
    self.call_exhaustive_callbacks();
  }
}
