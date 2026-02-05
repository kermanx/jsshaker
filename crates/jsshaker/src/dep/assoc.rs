use std::{
  collections::hash_map::Entry,
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
impl Default for EntityTrackerDep {
  fn default() -> Self {
    Self(COUNTER.fetch_add(1, Ordering::Relaxed))
  }
}

impl<'a> CustomDepTrait<'a> for EntityTrackerDep {
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    // let entities = analyzer.assoc_deps.to_entities.entry(*self).or_default();
    // analyzer.consume(entities.take());
    match analyzer.assoc_deps.to_entities.entry(*self) {
      Entry::Vacant(e) => {
        e.insert(None);
      }
      Entry::Occupied(mut e) => {
        let entities = e.get_mut().take();
        analyzer.consume(entities);
      }
    }
  }
}

#[derive(Debug, Default)]
pub struct AssocDepMap<'a> {
  to_deps: FxHashMap<DepAtom, Vec<Dep<'a>>>,
  to_entities: FxHashMap<EntityTrackerDep, Option<Vec<Entity<'a>>>>,
}

impl<'a> Analyzer<'a> {
  pub fn add_assoc_dep(&mut self, base: impl Into<DepAtom>, dep: Dep<'a>) {
    self.assoc_deps.to_deps.entry(base.into()).or_default().push(dep);
  }

  pub fn add_assoc_entity_dep(&mut self, base: EntityTrackerDep, entity: Entity<'a>) {
    match self.assoc_deps.to_entities.entry(base) {
      Entry::Vacant(e) => {
        e.insert(Some(vec![entity]));
      }
      Entry::Occupied(mut e) => {
        if let Some(vec) = e.get_mut() {
          vec.push(entity);
        } else {
          self.consume(entity);
        }
      }
    }
  }

  pub fn post_analyze_handle_assoc_deps(&mut self) -> bool {
    let mut to_consume = vec![];
    self.assoc_deps.to_deps.retain(|base, deps| {
      if self.included_atoms.is_included(*base) {
        to_consume.push(mem::take(deps));
        false
      } else {
        true
      }
    });
    let changed = !to_consume.is_empty();
    self.consume(to_consume);

    changed
  }
}
