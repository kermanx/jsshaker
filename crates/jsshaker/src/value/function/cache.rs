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

type FnOuterDeps<'a> = allocator::HashMap<'a, ReadWriteTarget<'a>, EntityOrTDZ<'a>>;

#[derive(Debug, Default)]
pub struct FnCacheTrackingData<'a> {
  // pub outer_deps: allocator::HashSet<'a, ReadWriteTarget<'a>>,
  pub outer_deps: Option<FnOuterDeps<'a>>,
}

impl<'a> FnCacheTrackingData<'a> {
  pub fn worst_case() -> Self {
    Self::default()
  }

  pub fn new_in(alloc: &'a allocator::Allocator) -> Self {
    Self { outer_deps: Some(allocator::HashMap::new_in(alloc)) }
  }

  pub fn track_read(&mut self, target: ReadWriteTarget<'a>, cachable: TrackReadCachable<'a>) {
    if let TrackReadCachable::Mutable(current_value) = cachable
      && let Some(outer_deps) = self.outer_deps.as_mut()
    {
      if outer_deps.len() > 8 {
        self.outer_deps = None;
        return;
      }
      match outer_deps.entry(target) {
        allocator::hash_map::Entry::Occupied(v) => {
          if match (*v.get(), current_value) {
            (Some(e1), Some(e2)) => !e1.exactly_same(e2),
            (None, None) => false,
            _ => true,
          } {
            self.outer_deps = None;
          }
        }
        allocator::hash_map::Entry::Vacant(v) => {
          v.insert(current_value);
        }
      }
    }
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
  pub outer_deps: FnOuterDeps<'a>,
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

  pub fn get_ret(&self, analyzer: &Analyzer<'a>, key: &FnCacheEntryKey<'a>) -> Option<Entity<'a>> {
    // self.table.get(key).map(|v| v.ret.into_entity(analyzer))
    if let Some(cached) = self.table.get(key) {
      for (&target, &last_value) in &cached.outer_deps {
        let current_value = analyzer.get_rw_target_current_value(target);
        if match (last_value, current_value) {
          (Some(e1), Some(e2)) => !e1.exactly_same(e2),
          (None, None) => false,
          _ => true,
        } {
          return None;
        }
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
    let Some(outer_deps) = tracking.outer_deps else {
      return;
    };
    let Some(ret) = ret.as_cachable() else {
      return;
    };
    self.table.insert(key, FnCacheEntryValue { outer_deps, ret });
  }
}
