use std::{cell::Cell, fmt::Debug, mem};

use oxc::allocator;
use rustc_hash::FxHashMap;

use crate::{
  analyzer::Analyzer,
  dep::{CustomDepTrait, Dep, DepAtom},
  entity::Entity,
  scope::CfScopeKind,
  transformer::Transformer,
  utils::ast::AstKind2,
};

#[derive(Debug, Default)]
struct ConditionalData<'a> {
  maybe_true: bool,
  maybe_false: bool,
  impure_true: bool,
  impure_false: bool,
  tests_to_include: Vec<Entity<'a>>,
}

#[derive(Debug, Default)]
pub struct ConditionalDataMap<'a> {
  callsite_to_branches: FxHashMap<AstKind2<'a>, Vec<&'a ConditionalBranch<'a>>>,
  node_to_data: FxHashMap<DepAtom, ConditionalData<'a>>,
}

#[derive(Debug, Clone)]
struct ConditionalBranch<'a> {
  id: DepAtom,
  is_true_branch: bool,
  maybe_true: bool,
  maybe_false: bool,
  test: Entity<'a>,
  included: &'a Cell<bool>,
}

impl<'a> ConditionalBranch<'a> {
  fn include_with_data(&self, data: &mut ConditionalData<'a>) {
    if self.included.replace(true) {
      return;
    }
    data.maybe_true |= self.maybe_true;
    data.maybe_false |= self.maybe_false;
    data.tests_to_include.push(self.test);
    if self.is_true_branch {
      data.impure_true = true;
    } else {
      data.impure_false = true;
    }
  }
}

impl<'a> CustomDepTrait<'a> for ConditionalBranch<'a> {
  fn include(&self, analyzer: &mut Analyzer<'a>) {
    let data = analyzer.get_conditional_data_mut(self.id);
    self.include_with_data(data);
  }
}

impl<'a> Analyzer<'a> {
  #[allow(clippy::too_many_arguments)]
  pub fn push_if_like_branch_cf_scope(
    &mut self,
    id: impl Into<DepAtom>,
    kind: CfScopeKind<'a>,
    test_prim: Entity<'a>,
    maybe_consequent: bool,
    maybe_alternate: bool,
    is_consequent: bool,
    has_contra: bool,
  ) -> Option<Dep<'a>> {
    self.push_conditional_cf_scope(
      id,
      kind,
      test_prim,
      maybe_consequent,
      maybe_alternate,
      is_consequent,
      has_contra,
    )
  }

  pub fn forward_logical_left_val(
    &mut self,
    id: impl Into<DepAtom>,
    left: Entity<'a>,
    left_prim: Entity<'a>,
    maybe_left: bool,
    maybe_right: bool,
  ) -> Entity<'a> {
    assert!(maybe_left);
    let dep = self.register_conditional_data(id, left_prim, maybe_left, maybe_right, true, true);
    self.factory.computed(left, dep)
  }

  pub fn push_logical_right_cf_scope(
    &mut self,
    id: impl Into<DepAtom>,
    left_prim: Entity<'a>,
    maybe_left: bool,
    maybe_right: bool,
  ) -> Option<Dep<'a>> {
    assert!(maybe_right);
    self.push_conditional_cf_scope(
      id,
      CfScopeKind::NonDet,
      left_prim,
      maybe_left,
      maybe_right,
      false,
      false,
    )
  }

  #[allow(clippy::too_many_arguments)]
  fn push_conditional_cf_scope(
    &mut self,
    id: impl Into<DepAtom>,
    kind: CfScopeKind<'a>,
    test_prim: Entity<'a>,
    maybe_true: bool,
    maybe_false: bool,
    is_true: bool,
    has_contra: bool,
  ) -> Option<Dep<'a>> {
    let dep =
      self.register_conditional_data(id, test_prim, maybe_true, maybe_false, is_true, has_contra);

    let non_det = maybe_true && maybe_false;

    self.push_cf_scope_with_deps(kind, allocator::Vec::from_iter_in(dep, self.allocator), non_det);

    dep
  }

  fn register_conditional_data(
    &mut self,
    id: impl Into<DepAtom>,
    test: Entity<'a>,
    maybe_true: bool,
    maybe_false: bool,
    is_true: bool,
    has_contra: bool,
  ) -> Option<Dep<'a>> {
    let id = id.into();
    let callsite = self.call_scope().callsite;

    let ConditionalDataMap { callsite_to_branches, node_to_data } = &mut self.conditional_data;

    let data = node_to_data.entry(id).or_insert_with(ConditionalData::default);

    if data.maybe_true && data.maybe_false {
      self.include(test);
      return None;
    }

    let branch = self.allocator.alloc(ConditionalBranch {
      id,
      is_true_branch: is_true,
      maybe_true,
      maybe_false,
      test,
      included: self.allocator.alloc(Cell::new(false)),
    });

    if has_contra {
      callsite_to_branches.entry(callsite).or_insert_with(Default::default).push(branch);
    }

    Some(Dep(branch))
  }

  pub fn post_analyze_handle_conditional(&mut self) -> bool {
    let included_atoms = &self.included_atoms;
    let ConditionalDataMap { callsite_to_branches, node_to_data } = &mut self.conditional_data;

    callsite_to_branches.retain(|callsite, branches| {
      if included_atoms.is_included(*callsite) {
        let mut remaining_branches = vec![];
        for branch in branches {
          let data = node_to_data.get_mut(&branch.id).unwrap();
          let is_opposite_impure =
            if branch.is_true_branch { data.impure_false } else { data.impure_true };
          if is_opposite_impure {
            branch.include_with_data(data);
          } else {
            remaining_branches.push(branch);
          }
        }
        !remaining_branches.is_empty()
      } else {
        true
      }
    });

    let mut tests_to_include = vec![];
    for data in node_to_data.values_mut() {
      if data.maybe_true && data.maybe_false {
        tests_to_include.push(mem::take(&mut data.tests_to_include));
      }
    }

    let mut dirty = false;
    for tests in tests_to_include {
      for test in tests {
        test.include(self);
        dirty = true;
      }
    }
    dirty
  }

  fn get_conditional_data_mut(&mut self, id: DepAtom) -> &mut ConditionalData<'a> {
    self.conditional_data.node_to_data.get_mut(&id).unwrap()
  }
}

impl Transformer<'_> {
  pub fn get_conditional_result(
    &self,
    id: impl Into<DepAtom>,
    accept_not_found: bool,
  ) -> (bool, bool, bool) {
    let id = id.into();
    let Some(data) = &self.conditional_data.node_to_data.get(&id) else {
      debug_assert!(accept_not_found, "Conditional result not found for {:?} {}", id, self.path);
      return (false, false, false);
    };

    if data.maybe_true && data.maybe_false {
      debug_assert!(data.tests_to_include.is_empty());
    }
    (data.maybe_true && data.maybe_false, data.maybe_true, data.maybe_false)
  }

  pub fn get_chain_result(
    &self,
    id: impl Into<DepAtom>,
    optional: bool,
    need_val: bool,
  ) -> (bool, bool) {
    if optional {
      let (need_optional, _, may_not_short_circuit) = self.get_conditional_result(id, !need_val);
      (need_optional, !may_not_short_circuit)
    } else {
      (false, false)
    }
  }
}
