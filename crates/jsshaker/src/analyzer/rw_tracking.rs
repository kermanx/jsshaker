use oxc::semantic::SymbolId;

use crate::{
  Analyzer,
  dep::EntityTrackerDep,
  entity::Entity,
  module::ModuleId,
  scope::{CfScopeId, VariableScopeId, variable_scope::EntityOrTDZ},
  value::{ObjectId, PropertyKeyValue, array::ArrayId},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReadWriteTarget<'a> {
  Variable(VariableScopeId, SymbolId),
  DefaultExport(ModuleId),
  ObjectAll(ObjectId),
  ObjectField(ObjectId, PropertyKeyValue<'a>),
  __Object(ObjectId),
  Array(ArrayId),
}

impl<'a> ReadWriteTarget<'a> {
  pub fn object_read_extra(self) -> Option<ReadWriteTarget<'a>> {
    match self {
      ReadWriteTarget::ObjectField(id, _) => Some(ReadWriteTarget::__Object(id)),
      _ => None,
    }
  }

  pub fn object_write_extra(self) -> Option<ReadWriteTarget<'a>> {
    match self {
      ReadWriteTarget::ObjectAll(id) => Some(ReadWriteTarget::__Object(id)),
      ReadWriteTarget::ObjectField(id, _) => Some(ReadWriteTarget::ObjectAll(id)),
      _ => None,
    }
  }
}

#[derive(Debug, Clone, Copy)]
pub enum TrackReadCacheable<'a> {
  Immutable,
  Mutable(EntityOrTDZ<'a>),
}

impl<'a> Analyzer<'a> {
  pub fn track_read(
    &mut self,
    scope: CfScopeId,
    target: ReadWriteTarget<'a>,
    cacheable: Option<TrackReadCacheable<'a>>,
  ) -> Option<EntityTrackerDep> {
    let target_depth = self.find_first_different_cf_scope(scope);
    let mut registered = false;
    let mut tracker_dep = None;
    for depth in (target_depth..self.scoping.cf.stack_len()).rev() {
      let scope = self.scoping.cf.data_at_mut(depth);
      if let Some(data) = scope.exhaustive_data_mut() {
        if data.clean
          && let Some(temp_deps) = data.temp_deps.as_mut()
        {
          temp_deps.insert(target);
          target.object_read_extra().map(|id| temp_deps.insert(id));
        }
        if !registered && let Some(register_deps) = data.register_deps.as_mut() {
          registered = true;
          register_deps.insert(target);
          target.object_read_extra().map(|id| register_deps.insert(id));
        }
      }
      if let Some(data) = scope.fn_cache_tracking_data_mut() {
        data.track_read(&mut self.assoc_deps, target, cacheable, &mut tracker_dep);
      }
    }
    tracker_dep
  }

  /// Returns `Some` for exhaustive write tracking, `Some(true)` if exhausted_forever
  pub fn track_write(
    &mut self,
    scope_depth: usize,
    target: ReadWriteTarget<'a>,
    mut cacheable: Option<Entity<'a>>,
  ) -> Option<bool> {
    if let Some(c) = cacheable
      && !c.as_cacheable(self).is_some_and(|c| c.is_copyable())
    {
      cacheable = None;
    }

    let mut exhaustive_drain = false;
    let mut exhaustive_register = false;
    let mut must_mark = true;
    let mut has_fn_scope = false;
    for depth in scope_depth..self.scoping.cf.stack_len() {
      let scope = self.scoping.cf.data_at_mut(depth);
      has_fn_scope |= scope.kind.is_function();
      if let Some(data) = scope.exhaustive_data_mut() {
        exhaustive_drain |= data.drain;
        exhaustive_register |= data.register;
        if (must_mark || data.register_deps.is_some())
          && data.clean
          && let Some(temp_deps) = &data.temp_deps
        {
          if temp_deps.contains(&target)
            || target.object_write_extra().is_some_and(|t| temp_deps.contains(&t))
          {
            data.clean = false;
          }
          must_mark = false;
        }
      }
    }
    if has_fn_scope {
      let mut non_det = false;
      for depth in (scope_depth..self.scoping.cf.stack_len()).rev() {
        let scope = self.scoping.cf.data_at_mut(depth);
        non_det |= scope.non_det();
        if let Some(data) = scope.fn_cache_tracking_data_mut() {
          data.track_write(target, cacheable.map(|e| (non_det, e)));
        }
      }
    }
    exhaustive_drain.then_some(exhaustive_register)
  }

  pub fn get_rw_target_current_value(&self, target: ReadWriteTarget<'a>) -> EntityOrTDZ<'a> {
    match target {
      ReadWriteTarget::Variable(scope, symbol) => {
        let scope = self.scoping.variable.get(scope);
        let variable = scope.variables[&symbol].borrow();
        let value = variable.value;
        if let Some(dep) = variable.exhausted {
          Some(self.factory.computed_unknown(dep))
        } else {
          value
        }
      }
      _ => unreachable!(),
    }
  }

  pub fn set_rw_target_current_value(
    &mut self,
    target: ReadWriteTarget<'a>,
    value: Entity<'a>,
    non_det: bool,
  ) {
    match target {
      ReadWriteTarget::Variable(scope, symbol) => {
        let written = self.write_on_scope(scope, symbol, value, non_det);
        debug_assert!(written);
      }
      _ => unreachable!(),
    }
  }
}
