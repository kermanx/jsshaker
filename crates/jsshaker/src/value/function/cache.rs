use oxc::allocator;

use crate::{
  Analyzer,
  entity::Entity,
  value::{ArgumentsValue, cachable::Cachable},
};

#[derive(Debug, Clone, Copy)]
pub struct FnCacheTrackingData {
  // pub outer_deps: allocator::HashSet<'a, ReadWriteTarget<'a>>,
  pub has_outer_deps: bool,
}

impl FnCacheTrackingData {
  pub fn worst_case() -> Self {
    Self { has_outer_deps: true }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FnCacheEntryKey<'a> {
  pub is_ctor: bool,
  pub this: Cachable<'a>,
  pub args: &'a [Cachable<'a>],
}

#[derive(Debug)]
pub struct FnCachentryValue<'a> {
  pub ret: Cachable<'a>,
}

#[derive(Debug)]
pub struct FnCache<'a> {
  table: allocator::HashMap<'a, FnCacheEntryKey<'a>, FnCachentryValue<'a>>,
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
    let this = this.as_cachable()?;
    args.rest?; // TODO: Support this case
    let mut cargs = analyzer.factory.vec();
    for arg in args.elements {
      cargs.push(arg.as_cachable()?);
    }
    Some(FnCacheEntryKey { is_ctor: IS_CTOR, this, args: cargs.into_bump_slice() })
  }

  pub fn get_ret(&self, analyzer: &Analyzer<'a>, key: &FnCacheEntryKey<'a>) -> Option<Entity<'a>> {
    self.table.get(key).map(|v| v.ret.into_entity(analyzer))
  }

  pub fn update_cache(
    &mut self,
    key: FnCacheEntryKey<'a>,
    ret: Entity<'a>,
    tracking: FnCacheTrackingData,
  ) {
    if tracking.has_outer_deps {
      return;
    }
    let Some(ret) = ret.as_cachable() else {
      return;
    };
    self.table.insert(key, FnCachentryValue { ret });
  }
}
