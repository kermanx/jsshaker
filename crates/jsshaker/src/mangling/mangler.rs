use oxc::{
  allocator::{self, Allocator},
  span::Atom,
};
use oxc_index::IndexVec;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
  analyzer::Factory,
  dep::DepAtom,
  utils::box_bump::BoxBump,
  value::{LiteralValue, Value},
};

use super::{MangleAtom, utils::get_mangled_name};

oxc_index::define_index_type! {
  pub struct IdentityGroupId = u32;
  DISABLE_MAX_INDEX_CHECK = cfg!(not(debug_assertions));
}

oxc_index::define_index_type! {
  pub struct UniquenessGroupId = u32;
  DISABLE_MAX_INDEX_CHECK = cfg!(not(debug_assertions));
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UniquenessConstraint<'a> {
  Constant(&'a str),
  Group(UniquenessGroupId),
}

#[derive(Debug)]
pub enum AtomState<'a> {
  Constrained(&'a str, Option<IdentityGroupId>, allocator::HashSet<'a, UniquenessConstraint<'a>>),
  Constant(&'a str),
  Builtin,
}

type UniquenessGroups<'a> =
  IndexVec<UniquenessGroupId, (Vec<MangleAtom>, usize, FxHashSet<&'a str>)>;

pub struct Mangler<'a> {
  pub enabled: bool,

  pub allocator: &'a Allocator,

  pub states: BoxBump<'a, MangleAtom, AtomState<'a>>,
  pub constant_nodes: FxHashMap<DepAtom, (Option<MangleAtom>, Value<'a>)>,

  /// (atoms, resolved_name)[]
  pub identity_groups: IndexVec<IdentityGroupId, (Vec<MangleAtom>, Option<&'a str>)>,
  /// (atoms, used_names, constant_names)[]
  pub uniqueness_groups: UniquenessGroups<'a>,
}

impl<'a> Mangler<'a> {
  pub fn new(enabled: bool, factory: &mut Factory<'a>) -> Self {
    let allocator = factory.allocator;
    let states = BoxBump::new(allocator);
    Self {
      enabled,
      allocator,
      states,
      constant_nodes: FxHashMap::default(),
      identity_groups: IndexVec::new(),
      uniqueness_groups: IndexVec::new(),
    }
  }

  pub fn use_constant_node(&mut self, node: impl Into<DepAtom>, str: &'a Atom<'a>) -> Value<'a> {
    self
      .constant_nodes
      .entry(node.into())
      .or_insert_with(|| {
        let atom = self.states.alloc(AtomState::Constrained(
          str,
          None,
          allocator::HashSet::new_in(self.allocator),
        ));
        (Some(atom), self.allocator.alloc(LiteralValue::String(str, Some(atom))))
      })
      .1
  }

  pub fn new_atom(&self, str: &'a str) -> MangleAtom {
    self.states.alloc(AtomState::Constrained(str, None, allocator::HashSet::new_in(self.allocator)))
  }

  pub fn new_constant_atom(&self, str: &'a str) -> MangleAtom {
    self.states.alloc(AtomState::Constant(str))
  }

  pub fn resolve(&mut self, atom: MangleAtom) -> Option<&'a str> {
    if !self.enabled {
      return None;
    }
    match &self.states[atom] {
      AtomState::Constrained(_, identity_group, uniq_constraints) => {
        let resolved = if let Some(identity_group) = identity_group {
          self.resolve_identity_group(*identity_group)
        } else if uniq_constraints.is_empty() {
          // This is quite weird, isn't it?
          "_"
        } else {
          let name = resolve_uniqueness_constrains(&mut self.uniqueness_groups, uniq_constraints);
          self.allocator.alloc_str(&name)
        };
        self.states[atom] = AtomState::Constant(resolved);
        Some(resolved)
      }
      AtomState::Constant(name) => Some(*name),
      AtomState::Builtin => None,
    }
  }

  pub fn resolve_node(&mut self, node: impl Into<DepAtom>) -> Option<&'a str> {
    if let Some(atom) = self.constant_nodes.get(&node.into()).and_then(|&(a, _)| a) {
      self.resolve(atom)
    } else {
      None
    }
  }

  fn resolve_identity_group(&mut self, id: IdentityGroupId) -> &'a str {
    let Mangler { identity_groups, uniqueness_groups, states, .. } = self;
    let (atoms, resolved_name) = &mut identity_groups[id];
    resolved_name.get_or_insert_with(|| {
      let mut uniq_constraints = FxHashSet::default();
      let mut constant = None;
      for atom in atoms {
        match &states[*atom] {
          AtomState::Constrained(_, _, uniq_groups) => {
            uniq_constraints.extend(uniq_groups);
          }
          AtomState::Constant(s) => constant = Some(*s),
          AtomState::Builtin => {}
        }
      }
      if let Some(constant) = constant {
        for c in uniq_constraints {
          if let UniquenessConstraint::Group(group) = c {
            uniqueness_groups[group].2.insert(constant);
          }
        }
        return constant;
      }
      let name = resolve_uniqueness_constrains(uniqueness_groups, &uniq_constraints);
      self.allocator.alloc_str(&name)
    })
  }
}

fn resolve_uniqueness_constrains<'a: 'b, 'b, I>(
  uniqueness_groups: &mut UniquenessGroups<'a>,
  constraints: &'b I,
) -> String
where
  &'b I: IntoIterator<Item = &'b UniquenessConstraint<'a>>,
{
  let mut n = constraints
    .into_iter()
    .map(|&c| match c {
      UniquenessConstraint::Constant(_) => 0,
      UniquenessConstraint::Group(group) => uniqueness_groups[group].1,
    })
    .max()
    .unwrap_or(0);
  let (n, name) = loop {
    let name = get_mangled_name(n);
    if constraints.into_iter().all(|&c| match c {
      UniquenessConstraint::Constant(s) => s != name.as_str(),
      UniquenessConstraint::Group(group) => !uniqueness_groups[group].2.contains(name.as_str()),
    }) {
      break (n, name);
    }
    n += 1;
  };
  for c in constraints {
    if let UniquenessConstraint::Group(group) = c {
      uniqueness_groups[*group].1 = n + 1;
    }
  }
  name
}
