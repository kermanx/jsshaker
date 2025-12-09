use oxc::allocator;

use crate::{
  Analyzer,
  entity::Entity,
  scope::{
    rw_tracking::{ReadWriteTarget, TrackReadCachable},
    variable_scope::EntityOrTDZ,
  },
  value::{ArgumentsValue, cachable::Cachable},
};

type FnReadDeps<'a> = allocator::HashMap<'a, ReadWriteTarget<'a>, EntityOrTDZ<'a>>;
type FnWriteEffects<'a> = allocator::HashMap<'a, ReadWriteTarget<'a>, (bool, Entity<'a>)>;

#[derive(Debug, Default)]
pub enum FnCacheTrackingData<'a> {
  #[default]
  UnTrackable,
  Tracked {
    read_deps: FnReadDeps<'a>,
    write_effects: FnWriteEffects<'a>,
  },
}

impl<'a> FnCacheTrackingData<'a> {
  pub fn worst_case() -> Self {
    Self::default()
  }

  pub fn new_in(alloc: &'a allocator::Allocator) -> Self {
    Self::Tracked {
      read_deps: allocator::HashMap::new_in(alloc),
      write_effects: allocator::HashMap::new_in(alloc),
    }
  }

  pub fn track_read(
    &mut self,
    target: ReadWriteTarget<'a>,
    cachable: Option<TrackReadCachable<'a>>,
  ) {
    let Self::Tracked { read_deps, .. } = self else {
      return;
    };
    let Some(cachable) = cachable else {
      *self = Self::UnTrackable;
      return;
    };
    let TrackReadCachable::Mutable(current_value) = cachable else {
      return;
    };
    if read_deps.len() > 8 {
      *self = Self::UnTrackable;
      return;
    }
    match read_deps.entry(target) {
      allocator::hash_map::Entry::Occupied(v) => {
        // TODO: Remove these?
        if match (*v.get(), current_value) {
          (Some(e1), Some(e2)) => !e1.exactly_same(e2),
          (None, None) => false,
          _ => true,
        } {
          *self = Self::UnTrackable;
        }
      }
      allocator::hash_map::Entry::Vacant(v) => {
        v.insert(current_value);
      }
    }
  }

  pub fn track_write(&mut self, target: ReadWriteTarget<'a>, cachable: Option<(bool, Entity<'a>)>) {
    let Self::Tracked { write_effects, .. } = self else {
      return;
    };
    let Some(cachable) = cachable else {
      *self = Self::UnTrackable;
      return;
    };
    write_effects.insert(target, cachable);
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FnCacheEntryKey<'a> {
  pub is_ctor: bool,
  pub this: Cachable<'a>,
  pub args: &'a [Cachable<'a>],
}

#[derive(Debug)]
pub struct FnCacheEntryValue<'a> {
  pub read_deps: FnReadDeps<'a>,
  pub write_effects: FnWriteEffects<'a>,
  pub ret: Cachable<'a>,
}

#[derive(Debug)]
pub struct FnCache<'a> {
  table: allocator::HashMap<'a, FnCacheEntryKey<'a>, FnCacheEntryValue<'a>>,
}

impl<'a> FnCache<'a> {
  pub fn new_in(alloc: &'a allocator::Allocator) -> Self {
    Self { table: allocator::HashMap::new_in(alloc) }
  }

  pub fn get_key<const IS_CTOR: bool>(
    analyzer: &Analyzer<'a>,
    this: Entity<'a>,
    args: ArgumentsValue<'a>,
  ) -> Option<FnCacheEntryKey<'a>> {
    if !analyzer.config.enable_fn_cache {
      return None;
    }

    let this = this.as_cachable()?;
    if args.rest.is_some() {
      return None; // TODO: Support this case
    }
    let mut cargs = analyzer.factory.vec();
    for arg in args.elements {
      cargs.push(arg.as_cachable()?);
    }
    Some(FnCacheEntryKey { is_ctor: IS_CTOR, this, args: cargs.into_bump_slice() })
  }

  pub fn retrive(
    &self,
    analyzer: &mut Analyzer<'a>,
    key: &FnCacheEntryKey<'a>,
  ) -> Option<Entity<'a>> {
    if let Some(cached) = self.table.get(key) {
      for (&target, &last_value) in &cached.read_deps {
        let current_value = analyzer.get_rw_target_current_value(target);
        if match (last_value, current_value) {
          (Some(e1), Some(e2)) => !e1.exactly_same(e2),
          (None, None) => false,
          _ => true,
        } {
          return None;
        }
      }

      for (&target, &(indeterminate, cachable)) in &cached.write_effects {
        analyzer.set_rw_target_current_value(target, cachable, indeterminate);
      }

      Some(cached.ret.into_entity(analyzer))
    } else {
      None
    }
  }

  pub fn update_cache(
    &mut self,
    key: FnCacheEntryKey<'a>,
    ret: Entity<'a>,
    tracking: FnCacheTrackingData<'a>,
  ) {
    let FnCacheTrackingData::Tracked { read_deps, write_effects } = tracking else {
      return;
    };
    let Some(ret) = ret.as_cachable() else {
      return;
    };
    self.table.insert(key, FnCacheEntryValue { read_deps, write_effects, ret });
  }
}
