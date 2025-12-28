use std::cell::RefCell;

use oxc::allocator;

use crate::{
  Analyzer,
  analyzer::rw_tracking::{ReadWriteTarget, TrackReadCachable},
  dep::{Dep, DepAtom},
  entity::Entity,
  scope::variable_scope::EntityOrTDZ,
  value::{ArgumentsValue, cacheable::Cacheable, call::FnCallInfo},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FnCachedInput<'a> {
  pub is_ctor: bool,
  pub this: &'a Cacheable<'a>,
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
pub struct FnCacheTrackDeps<'a> {
  call_id: DepAtom,
  this: DepAtom,
  args: &'a [DepAtom],
  rest: Option<DepAtom>,
}

impl<'a> FnCacheTrackDeps<'a> {
  pub fn wrap(
    analyzer: &Analyzer<'a>,
    call_id: DepAtom,
    this: &mut Entity<'a>,
    args: &mut ArgumentsValue<'a>,
  ) -> Self {
    let factory = analyzer.factory;
    let this_dep = DepAtom::from_counter();
    *this = factory.computed(*this, this_dep);
    let mut arg_deps = allocator::Vec::with_capacity_in(args.elements.len(), factory.allocator);
    let mut new_args = allocator::Vec::with_capacity_in(args.elements.len(), factory.allocator);
    for arg in args.elements {
      let arg_dep = DepAtom::from_counter();
      arg_deps.push(arg_dep);
      new_args.push(factory.computed(*arg, arg_dep));
    }
    args.elements = new_args.into_bump_slice();
    let rest_dep = if let Some(rest) = &mut args.rest {
      let rdep = DepAtom::from_counter();
      *rest = factory.computed(*rest, rdep);
      Some(rdep)
    } else {
      None
    };
    Self { call_id, this: this_dep, args: arg_deps.into_bump_slice(), rest: rest_dep }
  }
}

#[derive(Debug)]
pub struct FnCachedInfo<'a> {
  track_deps: FnCacheTrackDeps<'a>,
  effects: FnCachedEffects<'a>,
  has_global_effects: bool,
  ret: Entity<'a>,
}

#[derive(Debug)]
pub struct FnCache<'a> {
  table: RefCell<allocator::HashMap<'a, FnCachedInput<'a>, FnCachedInfo<'a>>>,
}

impl<'a> FnCache<'a> {
  pub fn new_in(alloc: &'a allocator::Allocator) -> Self {
    Self { table: allocator::HashMap::new_in(alloc).into() }
  }

  pub fn get_key<const IS_CTOR: bool>(
    analyzer: &Analyzer<'a>,
    this: Entity<'a>,
    args: ArgumentsValue<'a>,
  ) -> Option<FnCachedInput<'a>> {
    if !analyzer.config.enable_fn_cache {
      return None;
    }

    let this = analyzer.factory.alloc(this.as_cacheable(analyzer)?);
    if args.rest.is_some() {
      return None; // TODO: Support this case
    }
    let mut cargs = analyzer.factory.vec();
    for arg in args.elements {
      cargs.push(arg.as_cacheable(analyzer)?);
    }
    Some(FnCachedInput { is_ctor: IS_CTOR, this, args: cargs.into_bump_slice() })
  }

  pub fn retrieve(
    &self,
    analyzer: &mut Analyzer<'a>,
    key: &FnCachedInput<'a>,
    call_dep: Dep<'a>,
    this: Entity<'a>,
    args: ArgumentsValue<'a>,
  ) -> Option<Entity<'a>> {
    let table = self.table.borrow();
    if let Some(cached) = table.get(key) {
      let mut read_deps = None;
      for (&target, &last_value) in &cached.effects.reads {
        let current_value = analyzer.get_rw_target_current_value(target);
        match (last_value, current_value) {
          (Some(e1), Some(e2)) => {
            if !e1.exactly_same(e2) {
              let c1 = e1.as_cacheable(analyzer)?;
              let c2 = e2.as_cacheable(analyzer)?;
              if c1 != c2 {
                return None;
              }
              read_deps
                .get_or_insert_with(|| analyzer.factory.vec())
                .push(e2.get_shallow_dep(analyzer));
            }
          }
          (None, None) => {}
          _ => return None,
        }
      }
      let dep = analyzer.factory.dep((call_dep, read_deps));

      for (&target, &(indeterminate, cacheable)) in &cached.effects.writes {
        analyzer.set_rw_target_current_value(
          target,
          analyzer.factory.computed(cacheable, dep),
          indeterminate,
        );
      }

      analyzer.add_assoc_dep(cached.track_deps.call_id, dep);

      analyzer.add_assoc_entity_dep(cached.track_deps.this, this);
      for (dep, arg) in cached.track_deps.args.iter().zip(args.elements.iter()) {
        analyzer.add_assoc_entity_dep(*dep, *arg);
      }
      for (dep, rest) in cached.track_deps.rest.iter().zip(args.rest.iter()) {
        analyzer.add_assoc_entity_dep(*dep, *rest);
      }

      let ret = analyzer.factory.computed(cached.ret, dep);

      let has_global_effects = cached.has_global_effects;
      drop(table);
      if has_global_effects {
        analyzer.global_effect();
      }

      Some(ret)
    } else {
      None
    }
  }

  pub fn update(
    &self,
    analyzer: &Analyzer<'a>,
    key: FnCachedInput<'a>,
    ret: Entity<'a>,
    track_deps: FnCacheTrackDeps<'a>,
    tracking_data: FnCacheTrackingData<'a>,
    has_global_effects: bool,
  ) {
    let FnCacheTrackingData::Tracked { effects } = tracking_data else {
      return;
    };
    if ret.as_cacheable(analyzer).is_none() {
      return;
    };

    let mut table = self.table.borrow_mut();
    if table.len() < analyzer.config.fn_cache_size_limit {
      table.insert(key, FnCachedInfo { track_deps, effects, has_global_effects, ret });
    }
  }
}
