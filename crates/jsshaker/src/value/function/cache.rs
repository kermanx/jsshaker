use oxc::allocator::{self, Allocator};

use crate::{
  Analyzer,
  analyzer::rw_tracking::{ReadWriteTarget, TrackReadCacheable},
  entity::Entity,
  scope::variable_scope::EntityOrTDZ,
  value::{ArgumentsValue, FunctionValue, cacheable::Cacheable, call::FnCallInfo},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FnCachedInput<'a> {
  pub is_ctor: bool,
  pub this: Cacheable<'a>,
  pub args: &'a [Cacheable<'a>],
}

#[derive(Debug, Clone, Copy)]
pub struct FnCallEffect<'a> {
  pub func: &'a FunctionValue<'a>,
  pub input: FnCachedInput<'a>,
}
impl<'a> PartialEq for FnCallEffect<'a> {
  fn eq(&self, other: &Self) -> bool {
    std::ptr::eq(self.func, other.func) && self.input == other.input
  }
}
impl<'a> Eq for FnCallEffect<'a> {}
impl<'a> std::hash::Hash for FnCallEffect<'a> {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    (self.func as *const _, &self.input).hash(state);
  }
}

#[derive(Debug)]
pub struct FnCachedEffects<'a> {
  pub reads: allocator::HashMap<'a, ReadWriteTarget<'a>, EntityOrTDZ<'a>>,
  pub writes: allocator::HashMap<'a, ReadWriteTarget<'a>, (bool, Entity<'a>)>,
  pub calls: allocator::HashSet<'a, FnCallEffect<'a>>,
}

impl<'a> FnCachedEffects<'a> {
  pub fn new_in(allocator: &'a Allocator) -> Self {
    Self {
      reads: allocator::HashMap::new_in(allocator),
      writes: allocator::HashMap::new_in(allocator),
      calls: allocator::HashSet::new_in(allocator),
    }
  }
}

#[derive(Debug)]
pub enum FnCacheTrackingData<'a> {
  UnTrackable,
  Tracked { self_call_effect: FnCallEffect<'a>, effects: FnCachedEffects<'a> },
  Failed { self_call_effect: FnCallEffect<'a> },
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
    FnCacheTrackingData::UnTrackable
  }

  pub fn new_in(allocator: &'a allocator::Allocator, info: FnCallInfo<'a>) -> Self {
    if let Some(cache_key) = info.cache_key {
      Self::Tracked {
        self_call_effect: FnCallEffect { func: info.func, input: cache_key },
        effects: FnCachedEffects::new_in(allocator),
      }
    } else {
      FnCacheTrackingData::UnTrackable
    }
  }

  pub fn failed(&mut self) {
    let Self::Tracked { self_call_effect, .. } = self else {
      unreachable!();
    };
    *self = Self::Failed { self_call_effect: *self_call_effect };
  }

  pub fn call_effect(&self) -> Option<FnCallEffect<'a>> {
    match self {
      Self::Tracked { self_call_effect, .. } => Some(*self_call_effect),
      Self::Failed { self_call_effect } => Some(*self_call_effect),
      Self::UnTrackable => None,
    }
  }

  pub fn track_read(
    &mut self,
    target: ReadWriteTarget<'a>,
    cacheable: Option<TrackReadCacheable<'a>>,
  ) -> Option<FnCallEffect<'a>> {
    let Self::Tracked { self_call_effect, effects } = self else {
      return None;
    };
    let self_call_effect = *self_call_effect;
    let Some(cacheable) = cacheable else {
      self.failed();
      return Some(self_call_effect);
    };
    let TrackReadCacheable::Mutable(current_value) = cacheable else {
      return None;
    };
    if effects.reads.len() > 8 {
      self.failed();
      return Some(self_call_effect);
    }
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
    Some(self_call_effect)
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
      self.failed();
      return;
    };
    effects.writes.insert(target, cacheable);
  }

  pub fn track_call(&mut self, effect: FnCallEffect<'a>) {
    let Self::Tracked { effects, .. } = self else {
      return;
    };
    effects.calls.insert(effect);
  }
}

#[derive(Debug)]
pub struct FnCache<'a> {
  table: allocator::HashMap<'a, FnCachedInput<'a>, (FnCachedEffects<'a>, Cacheable<'a>)>,
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
    if let Some((cached, ret)) = self.table.get(key) {
      for (&target, &last_value) in &cached.reads {
        let current_value = analyzer.get_rw_target_current_value(target);
        if match (last_value, current_value) {
          (Some(e1), Some(e2)) => !e1.exactly_same(e2),
          (None, None) => false,
          _ => true,
        } {
          return None;
        }
      }

      for (&target, &(indeterminate, cacheable)) in &cached.writes {
        analyzer.set_rw_target_current_value(target, cacheable, indeterminate);
      }

      Some(ret.into_entity(analyzer))
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
    let FnCacheTrackingData::Tracked { effects, .. } = tracking else {
      return;
    };
    let Some(ret) = ret.as_cacheable() else {
      return;
    };
    self.table.insert(key, (effects, ret));
  }
}
