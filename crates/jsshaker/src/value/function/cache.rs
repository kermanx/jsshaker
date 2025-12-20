use oxc::allocator;

use crate::{
  Analyzer,
  analyzer::rw_tracking::{ReadWriteTarget, TrackReadCachable},
  entity::Entity,
  scope::variable_scope::EntityOrTDZ,
  value::{ArgumentsValue, cacheable::Cacheable, call::FnCallInfo},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FnCachedInput<'a> {
  pub is_ctor: bool,
  pub this: Cacheable<'a>,
  pub args: &'a [Cacheable<'a>],
}

#[derive(Debug)]
pub struct FnCachedEffects<'a> {
  pub reads: allocator::HashMap<'a, ReadWriteTarget<'a>, EntityOrTDZ<'a>>,
  pub writes: allocator::HashMap<'a, ReadWriteTarget<'a>, (bool, Entity<'a>)>,
}

#[derive(Debug, Default)]
pub enum FnCacheTrackingData<'a> {
  #[default]
  UnTrackable,
  Tracked {
    effects: FnCachedEffects<'a>,
  },
}

impl<'a> FnCachedEffects<'a> {
  pub fn new_in(allocator: &'a allocator::Allocator) -> Self {
    Self {
      reads: allocator::HashMap::new_in(allocator),
      writes: allocator::HashMap::new_in(allocator),
    }
  }
}

impl<'a> FnCacheTrackingData<'a> {
  pub fn worst_case() -> Self {
    Self::default()
  }

  pub fn new_in(allocator: &'a allocator::Allocator, info: FnCallInfo<'a>) -> Self {
    if let Some(_cache_key) = info.cache_key {
      Self::Tracked { effects: FnCachedEffects::new_in(allocator) }
    } else {
      FnCacheTrackingData::UnTrackable
    }
  }

  pub fn track_read(
    &mut self,
    target: ReadWriteTarget<'a>,
    cacheable: Option<TrackReadCachable<'a>>,
  ) {
    let Self::Tracked { effects, .. } = self else {
      return;
    };
    let Some(cacheable) = cacheable else {
      *self = Self::UnTrackable;
      return;
    };
    let TrackReadCachable::Mutable(current_value) = cacheable else {
      return;
    };
    match effects.reads.entry(target) {
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

  pub fn track_write(
    &mut self,
    target: ReadWriteTarget<'a>,
    cacheable: Option<(bool, Entity<'a>)>,
  ) {
    let Self::Tracked { effects, .. } = self else {
      return;
    };
    let Some(cacheable) = cacheable else {
      *self = Self::UnTrackable;
      return;
    };
    effects.writes.insert(target, cacheable);
  }
}

#[derive(Debug)]
pub struct FnCache<'a> {
  table: allocator::HashMap<'a, FnCachedInput<'a>, (FnCachedEffects<'a>, Entity<'a>)>,
}

impl<'a> FnCache<'a> {
  pub fn new_in(alloc: &'a allocator::Allocator) -> Self {
    Self { table: allocator::HashMap::new_in(alloc) }
  }

  pub fn get_key<const IS_CTOR: bool>(
    analyzer: &Analyzer<'a>,
    this: Entity<'a>,
    args: ArgumentsValue<'a>,
  ) -> Option<FnCachedInput<'a>> {
    if !analyzer.config.enable_fn_cache {
      return None;
    }

    let this = this.as_cacheable()?;
    if args.rest.is_some() {
      return None; // TODO: Support this case
    }
    let mut cargs = analyzer.factory.vec();
    for arg in args.elements {
      cargs.push(arg.as_cacheable()?);
    }
    Some(FnCachedInput { is_ctor: IS_CTOR, this, args: cargs.into_bump_slice() })
  }

  pub fn retrieve(
    &self,
    analyzer: &mut Analyzer<'a>,
    key: &FnCachedInput<'a>,
  ) -> Option<Entity<'a>> {
    if let Some((effects, ret)) = self.table.get(key) {
      for (&target, &last_value) in &effects.reads {
        let current_value = analyzer.get_rw_target_current_value(target);
        if match (last_value, current_value) {
          (Some(e1), Some(e2)) => !e1.exactly_same(e2),
          (None, None) => false,
          _ => true,
        } {
          return None;
        }
      }

      for (&target, &(indeterminate, cacheable)) in &effects.writes {
        analyzer.set_rw_target_current_value(target, cacheable, indeterminate);
      }

      Some(*ret)
    } else {
      None
    }
  }

  pub fn update_cache(
    &mut self,
    key: FnCachedInput<'a>,
    ret: Entity<'a>,
    tracking: FnCacheTrackingData<'a>,
  ) {
    let FnCacheTrackingData::Tracked { effects } = tracking else {
      return;
    };
    if ret.as_cacheable().is_none() {
      return;
    };
    self.table.insert(key, (effects, ret));
  }
}
