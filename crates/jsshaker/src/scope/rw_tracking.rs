use oxc::semantic::SymbolId;

use crate::{
  Analyzer,
  entity::Entity,
  scope::{CfScopeId, VariableScopeId, variable_scope::EntityOrTDZ},
  value::{ObjectId, PropertyKeyValue},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReadWriteTarget<'a> {
  Variable(VariableScopeId, SymbolId),
  ObjectAll(ObjectId),
  ObjectField(ObjectId, PropertyKeyValue<'a>),
  __Object(ObjectId),
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
pub enum TrackReadCachable<'a> {
  Immutable,
  Mutable(EntityOrTDZ<'a>),
}

impl<'a> Analyzer<'a> {
  pub fn track_read(
    &mut self,
    scope: CfScopeId,
    target: ReadWriteTarget<'a>,
    cachable: Option<TrackReadCachable<'a>>,
  ) {
    let target_depth = self.find_first_different_cf_scope(scope);
    let mut registered = false;
    for depth in (target_depth..self.scoping.cf.stack.len()).rev() {
      let scope = self.scoping.cf.get_mut_from_depth(depth);
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
        data.track_read(target, cachable);
      }
    }
  }

  pub fn track_write(&mut self, target: ReadWriteTarget<'a>, scope: usize) -> (bool, bool) {
    let mut exhaustive = false;
    let mut indeterminate = false;
    let mut need_mark = true;
    for depth in scope..self.scoping.cf.stack.len() {
      let scope = self.scoping.cf.get_mut_from_depth(depth);
      indeterminate |= scope.is_indeterminate();
      if let Some(data) = scope.exhaustive_data_mut() {
        exhaustive = true;
        if (need_mark || data.register_deps.is_some())
          && data.clean
          && let Some(temp_deps) = &data.temp_deps
        {
          if temp_deps.contains(&target) {
            data.clean = false;
          } else if let Some(id) = target.object_write_extra()
            && temp_deps.contains(&id)
          {
            data.clean = false;
          }
          need_mark = false;
        }
      }
      if let Some(data) = scope.fn_cache_tracking_data_mut() {
        data.track_read(target, None);
      }
    }
    (exhaustive, indeterminate)
  }

  pub fn get_rw_target_current_value(&self, target: ReadWriteTarget<'a>) -> Option<Entity<'a>> {
    match target {
      ReadWriteTarget::Variable(scope, symbol) => {
        let scope = self.scoping.variable.get(scope);
        scope.variables[&symbol].borrow().value
      }
      _ => unreachable!(),
    }
  }
}
