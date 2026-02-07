use std::{
  mem,
  sync::atomic::{AtomicUsize, Ordering},
};

use rustc_hash::FxHashMap;

use crate::{
  Analyzer,
  dep::{CustomDepTrait, Dep, DepAtom},
  entity::Entity,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityTrackerDep(usize);

static COUNTER: AtomicUsize = AtomicUsize::new(1);

impl<'a> CustomDepTrait<'a> for EntityTrackerDep {
  fn include(&self, analyzer: &mut Analyzer<'a>) {
    if let Some(entities) = analyzer.assoc_deps.to_entities.remove(self) {
      analyzer.include(entities);
    }
  }
}

#[derive(Debug, Default)]
pub struct AssocDepMap<'a> {
  to_deps: FxHashMap<DepAtom, Vec<Dep<'a>>>,
  to_entities: FxHashMap<EntityTrackerDep, Vec<Entity<'a>>>,
}

impl<'a> AssocDepMap<'a> {
  pub fn alloc_entity_tracker(&mut self) -> EntityTrackerDep {
    let dep = EntityTrackerDep(COUNTER.fetch_add(1, Ordering::Relaxed));
    self.to_entities.insert(dep, vec![]);
    dep
  }
}

impl<'a> Analyzer<'a> {
  pub fn add_assoc_dep(&mut self, base: impl Into<DepAtom>, dep: Dep<'a>) {
    self.assoc_deps.to_deps.entry(base.into()).or_default().push(dep);
  }

  pub fn add_assoc_entity_dep(&mut self, base: EntityTrackerDep, entity: Entity<'a>) {
    if let Some(entities) = self.assoc_deps.to_entities.get_mut(&base) {
      entities.push(entity);
    } else {
      self.include(entity);
    }
  }

  pub fn post_analyze_handle_assoc_deps(&mut self) -> bool {
    let mut to_include = vec![];
    self.assoc_deps.to_deps.retain(|base, deps| {
      if self.included_atoms.is_included(*base) {
        to_include.push(mem::take(deps));
        false
      } else {
        true
      }
    });
    let changed = !to_include.is_empty();
    self.include(to_include);

    changed
  }
}
