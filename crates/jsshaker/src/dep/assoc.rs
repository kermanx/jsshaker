use std::mem;

use rustc_hash::FxHashMap;

use crate::{
  Analyzer,
  dep::{Dep, DepAtom},
  entity::Entity,
};

#[derive(Debug, Default)]
pub struct AssocDepMap<'a> {
  to_deps: FxHashMap<DepAtom, Vec<Dep<'a>>>,
  to_entities: FxHashMap<DepAtom, Vec<Entity<'a>>>,
}

impl<'a> Analyzer<'a> {
  pub fn add_assoc_dep(&mut self, base: impl Into<DepAtom>, dep: Dep<'a>) {
    self.assoc_deps.to_deps.entry(base.into()).or_default().push(dep);
  }

  pub fn add_assoc_entity_dep(&mut self, base: impl Into<DepAtom>, entity: Entity<'a>) {
    self.assoc_deps.to_entities.entry(base.into()).or_default().push(entity);
  }

  pub fn post_analyze_handle_assoc_deps(&mut self) -> bool {
    let mut changed = false;

    let mut to_consume = vec![];
    self.assoc_deps.to_deps.retain(|base, deps| {
      if self.included_atoms.is_included(*base) {
        to_consume.push(mem::take(deps));
        false
      } else {
        true
      }
    });
    changed |= !to_consume.is_empty();
    self.consume(to_consume);

    let mut to_consume = vec![];
    self.assoc_deps.to_entities.retain(|base, entities| {
      if self.included_atoms.is_included(*base) {
        to_consume.push(mem::take(entities));
        false
      } else {
        true
      }
    });
    changed |= !to_consume.is_empty();
    self.consume(to_consume);

    changed
  }
}
