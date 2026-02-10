use std::mem;

use rustc_hash::FxHashSet;

use super::{AtomState, MangleAtom};
use super::{Mangler, UniquenessGroupId};
use crate::mangling::UniquenessConstraint;
use crate::utils::get_two_mut_from_vec;
use crate::{analyzer::Analyzer, dep::CustomDepTrait};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MangleConstraint<'a> {
  None,
  Eq(MangleAtom, MangleAtom),
  Neq(MangleAtom, MangleAtom),
  Unique(UniquenessGroupId, MangleAtom),
  Multiple(&'a [MangleConstraint<'a>]),
}

impl<'a> MangleConstraint<'a> {
  pub fn equality(
    eq: bool,
    a: Option<MangleAtom>,
    b: Option<MangleAtom>,
  ) -> Option<MangleConstraint<'a>> {
    if let (Some(a), Some(b)) = (a, b) {
      Some(if eq { MangleConstraint::Eq(a, b) } else { MangleConstraint::Neq(a, b) })
    } else {
      None
    }
  }
}

impl<'a> CustomDepTrait<'a> for MangleConstraint<'a> {
  fn include(&self, analyzer: &mut Analyzer<'a>) {
    match self {
      MangleConstraint::None => {}
      MangleConstraint::Eq(a, b) => {
        analyzer.mangler.mark_equality(true, *a, *b);
      }
      MangleConstraint::Neq(a, b) => {
        analyzer.mangler.mark_equality(false, *a, *b);
      }
      MangleConstraint::Unique(g, a) => {
        analyzer.mangler.add_to_uniqueness_group(*g, *a);
      }
      MangleConstraint::Multiple(cs) => {
        analyzer.include(*cs);
      }
    }
  }
}

impl<'a> Mangler<'a> {
  fn mark_equality(&mut self, eq: bool, a: MangleAtom, b: MangleAtom) {
    if a == b {
      return;
    }

    let Mangler { states, identity_groups, uniqueness_groups, .. } = self;

    match states.get_two_mut(a, b) {
      (AtomState::Builtin, AtomState::Builtin) => {}
      (AtomState::Builtin, x) | (x, AtomState::Builtin) => {
        if eq {
          *x = AtomState::Builtin;
        }
        // If neq, do nothing because currently the preserved strings are builtin strings
        // which is long enough to not conflict with mangled strings.
      }
      (AtomState::Constant(a), AtomState::Constant(b)) => assert_eq!(a == b, eq),
      (AtomState::Constant(a), _) => {
        let s = *a;
        self.mark_atom_constant(eq, b, s);
      }
      (_, AtomState::Constant(b)) => {
        let s = *b;
        self.mark_atom_constant(eq, a, s);
      }
      (AtomState::Constrained(_, ea, ua), AtomState::Constrained(_, eb, ub)) => {
        if eq {
          match ((ea, ua), (eb, ub)) {
            ((Some(ia), _), (Some(ib), _)) => {
              if ia != ib {
                let ((ga, _), (gb, _)) = get_two_mut_from_vec(identity_groups, *ia, *ib);
                let a_is_larger = ga.len() > gb.len();
                let (from, to) = if a_is_larger {
                  *ib = *ia;
                  (gb, ga)
                } else {
                  *ia = *ib;
                  (ga, gb)
                };
                let index = *ia;
                for atom in mem::take(from) {
                  to.push(atom);
                  let AtomState::Constrained(_, group, _) = &mut states[atom] else {
                    unreachable!();
                  };
                  *group = Some(index);
                }
              }
            }
            ((Some(ia), _), (ib @ None, _)) => {
              *ib = Some(*ia);
              identity_groups[*ia].0.push(b);
            }
            ((ia @ None, _), (Some(ib), _)) => {
              *ia = Some(*ib);
              identity_groups[*ib].0.push(a);
            }
            ((ia @ None, _), (ib @ None, _)) => {
              let group = identity_groups.push((vec![a, b], None));
              *ia = Some(group);
              *ib = Some(group);
            }
          }
        } else {
          let group = uniqueness_groups.push((vec![a, b], 0, FxHashSet::default()));
          ua.insert(UniquenessConstraint::Group(group));
          ub.insert(UniquenessConstraint::Group(group));
        }
      }
    }
  }

  pub fn mark_atom_non_mangable(&mut self, atom: MangleAtom) {
    let state = &mut self.states[atom];
    if let AtomState::Constrained(s, _, _) = state {
      let s = *s;
      let AtomState::Constrained(_, identity_group, _) =
        mem::replace(state, AtomState::Constant(s))
      else {
        unreachable!()
      };

      if let Some(index) = identity_group {
        for atom in mem::take(&mut self.identity_groups[index].0) {
          self.mark_atom_constant(true, atom, s);
        }
      }
    }
  }

  pub fn mark_uniqueness_group_non_mangable(&mut self, group: UniquenessGroupId) {
    for atom in mem::take(&mut self.uniqueness_groups[group].0) {
      self.mark_atom_non_mangable(atom);
    }
  }

  pub fn add_to_uniqueness_group(&mut self, group: UniquenessGroupId, atom: MangleAtom) {
    match &mut self.states[atom] {
      AtomState::Constrained(_, _, uniqueness_groups) => {
        uniqueness_groups.insert(UniquenessConstraint::Group(group));
        self.uniqueness_groups[group].0.push(atom);
      }
      AtomState::Constant(s) => {
        self.uniqueness_groups[group].2.insert(s);
      }
      AtomState::Builtin => {
        // Do nothing, explained above
      }
    }
  }

  fn mark_atom_constant(&mut self, eq: bool, atom: MangleAtom, value: &'a str) {
    let Mangler { identity_groups, states, .. } = self;

    if matches!(states[atom], AtomState::Builtin) {
      return;
    }

    if eq {
      let atom = mem::replace(&mut states[atom], AtomState::Constant(value));
      if let AtomState::Constrained(s, Some(identity_group), _) = atom {
        assert_eq!(s, value);
        for atom in mem::take(&mut identity_groups[identity_group].0) {
          states[atom] = AtomState::Constant(value);
        }
      }
    } else if let AtomState::Constrained(s, _, uniqueness_groups) = &mut states[atom] {
      assert_ne!(s, &value);
      uniqueness_groups.insert(UniquenessConstraint::Constant(value));
    }
  }
}
