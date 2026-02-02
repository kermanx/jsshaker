use std::collections::hash_map;

use oxc::{
  allocator::{self, Allocator},
  span::Atom,
};
use oxc_index::IndexVec;
use rustc_hash::FxHashMap;

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

#[derive(Debug)]
pub enum AtomState<'a> {
  Constrained(Option<IdentityGroupId>, allocator::HashSet<'a, UniquenessGroupId>),
  Constant(&'a str),
  NonMangable,
  Preserved,
}

pub struct Mangler<'a> {
  pub enabled: bool,

  pub allocator: &'a Allocator,

  pub states: BoxBump<'a, MangleAtom, AtomState<'a>>,
  pub constant_nodes: FxHashMap<DepAtom, (Option<MangleAtom>, Value<'a>)>,

  /// (atoms, resolved_name)[]
  pub identity_groups: IndexVec<IdentityGroupId, (Vec<MangleAtom>, Option<&'a str>)>,
  /// (atoms, used_names)[]
  pub uniqueness_groups: IndexVec<UniquenessGroupId, (Vec<MangleAtom>, usize)>,
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
    match self.constant_nodes.entry(node.into()) {
      hash_map::Entry::Occupied(mut entry) => {
        let (atom, value) = entry.get_mut();
        if let Some(a) = atom
          && matches!(self.states[*a], AtomState::NonMangable)
        {
          *atom = None;
          *value = str;
        }
        *value
      }
      hash_map::Entry::Vacant(entry) => {
        let atom = self
          .states
          .alloc(AtomState::Constrained(None, allocator::HashSet::new_in(self.allocator)));
        let value = self.allocator.alloc(LiteralValue::String(str, Some(atom)));
        entry.insert((Some(atom), value));
        value
      }
    }
  }

  pub fn new_atom(&self) -> MangleAtom {
    self.states.alloc(AtomState::Constrained(None, allocator::HashSet::new_in(self.allocator)))
  }

  pub fn new_constant_atom(&self, str: &'a str) -> MangleAtom {
    self.states.alloc(AtomState::Constant(str))
  }

  pub fn resolve(&mut self, atom: MangleAtom) -> Option<&'a str> {
    if !self.enabled {
      return None;
    }
    match &self.states[atom] {
      AtomState::Constrained(identity_group, uniqueness_groups) => {
        let resolved = if let Some(identity_group) = identity_group {
          self.resolve_identity_group(*identity_group)
        } else if uniqueness_groups.is_empty() {
          // This is quite weird, isn't it?
          "_"
        } else {
          let n =
            uniqueness_groups.iter().map(|&index| self.uniqueness_groups[index].1).max().unwrap();
          let name = get_mangled_name(n);
          for &index in uniqueness_groups {
            self.uniqueness_groups[index].1 = n + 1;
          }
          self.allocator.alloc_str(&name)
        };
        self.states[atom] = AtomState::Constant(resolved);
        Some(resolved)
      }
      AtomState::Constant(name) => Some(*name),
      AtomState::NonMangable => None,
      AtomState::Preserved => None,
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
      let mut n = 0;
      let mut related_uniq_groups = vec![];
      for atom in atoms {
        match &states[*atom] {
          AtomState::Constrained(_, uniq_groups) => {
            for index in uniq_groups {
              related_uniq_groups.push(*index);
              n = n.max(uniqueness_groups[*index].1);
            }
          }
          AtomState::Constant(s) => return *s,
          AtomState::NonMangable => unreachable!(),
          AtomState::Preserved => {}
        }
      }
      let name = get_mangled_name(n);
      for index in related_uniq_groups {
        uniqueness_groups[index].1 = n + 1;
      }
      self.allocator.alloc_str(&name)
    })
  }
}
