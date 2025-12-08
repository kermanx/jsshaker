use oxc::allocator::{self, Allocator};

use super::{MangleAtom, utils::get_mangled_name};
use crate::{define_box_bump_idx, utils::box_bump::BoxBump};

define_box_bump_idx! {
  pub struct IdentityGroupId;
}

define_box_bump_idx! {
  pub struct UniquenessGroupId;
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

  pub atoms: BoxBump<'a, MangleAtom, AtomState<'a>>,
  pub builtin_atom: MangleAtom,

  /// (atoms, resolved_name)[]
  pub identity_groups:
    BoxBump<'a, IdentityGroupId, (allocator::Vec<'a, MangleAtom>, Option<&'a str>)>,
  /// (atoms, used_names)[]
  pub uniqueness_groups: BoxBump<'a, UniquenessGroupId, (allocator::Vec<'a, MangleAtom>, usize)>,
}

impl<'a> Mangler<'a> {
  pub fn new(enabled: bool, allocator: &'a Allocator) -> Self {
    let atoms = BoxBump::new(allocator);
    let builtin_atom = atoms.alloc(AtomState::Preserved);
    Self {
      enabled,
      allocator,
      atoms,
      builtin_atom,
      identity_groups: BoxBump::new(allocator),
      uniqueness_groups: BoxBump::new(allocator),
    }
  }

  pub fn new_atom(&mut self) -> MangleAtom {
    self.atoms.alloc(AtomState::Constrained(None, allocator::HashSet::new_in(self.allocator)))
  }

  pub fn new_constant_atom(&mut self, str: &'a str) -> MangleAtom {
    self.atoms.alloc(AtomState::Constant(str))
  }

  pub fn resolve(&mut self, atom: MangleAtom) -> Option<&'a str> {
    if !self.enabled {
      return None;
    }
    match &self.atoms[atom] {
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
        self.atoms[atom] = AtomState::Constant(resolved);
        Some(resolved)
      }
      AtomState::Constant(name) => Some(*name),
      AtomState::NonMangable => None,
      AtomState::Preserved => None,
    }
  }

  fn resolve_identity_group(&mut self, id: IdentityGroupId) -> &'a str {
    let Mangler { identity_groups, uniqueness_groups, atoms: constraints, .. } = self;
    let (atoms, resolved_name) = &mut identity_groups[id];
    resolved_name.get_or_insert_with(|| {
      let mut n = 0;
      let mut related_uniq_groups = vec![];
      for atom in atoms {
        match &constraints[*atom] {
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
