use oxc::allocator;

use super::ObjectValue;
use crate::{
  analyzer::{Analyzer, rw_tracking::ReadWriteTarget},
  dep::{Dep, DepVec},
  entity::Entity,
  mangling::MangleAtom,
  scope::CfScopeKind,
  value::{PropertyKeyValue, Value, escaped, object::ObjectPrototype},
};

pub(crate) struct GetPropertyContext<'a> {
  pub this: Value<'a>,
  pub key: Entity<'a>,
  pub values: Vec<Entity<'a>>,
  pub getters: Vec<Entity<'a>>,
  pub extra_deps: DepVec<'a>,
}

impl<'a> ObjectValue<'a> {
  pub fn get_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    this: Value<'a>,
    dep: Dep<'a>,
    key: Entity<'a>,
  ) -> Entity<'a> {
    if self.is_self_or_proto_consumed() {
      return escaped::get_property(self, analyzer, dep, key);
    }

    let mut mangable = false;
    let mut context = GetPropertyContext {
      this,
      key,
      values: vec![],
      getters: vec![],
      extra_deps: analyzer.factory.vec(),
    };
    let mut exhaustive_deps = Some(vec![]);

    let mut check_rest = false;
    let key_literals = key.get_literals(analyzer);
    if let Some(key_literals) = &key_literals {
      mangable = self.check_mangable(analyzer, key_literals);
      for &key_literal in key_literals {
        let (key_str, key_atom) = key_literal.into();

        if key_str.is_special_key() {
          return analyzer.factory.computed_unknown((self, dep, key));
        }

        if !self.get_keyed(analyzer, &mut context, key_str, key_atom, exhaustive_deps.as_mut()) {
          check_rest = true;
        }
      }
    } else {
      self.disable_mangling(analyzer);
      self.get_any_keyed(analyzer, &mut context);
      check_rest = true;
      exhaustive_deps = None;
    }

    if check_rest {
      if let Some(rest) = &self.rest {
        rest.borrow_mut().get(analyzer, &mut context, None);
        exhaustive_deps = None;
      } else {
        context.values.push(analyzer.factory.undefined);
      }
    }

    if self.get_unknown_keyed(analyzer, &mut context) {
      exhaustive_deps = None;
    }

    if !context.getters.is_empty() {
      let non_det = check_rest || !context.values.is_empty() || context.getters.len() > 1;
      analyzer.push_cf_scope_with_deps(
        CfScopeKind::Dependent,
        analyzer.factory.vec1(if mangable { dep } else { analyzer.dep((dep, key)) }),
        non_det,
      );
      for getter in context.getters {
        analyzer.cf_scope_mut().exited = if non_det { None } else { Some(false) };
        context.values.push(getter.call_as_getter(analyzer, analyzer.factory.no_dep, self.into()));
      }
      analyzer.pop_cf_scope();
    }

    if let Some(exhaustive_deps) = exhaustive_deps {
      for key in exhaustive_deps {
        analyzer.track_read(
          self.cf_scope,
          ReadWriteTarget::ObjectField(self.object_id(), key),
          None,
        );
      }
    } else {
      analyzer.track_read(self.cf_scope, ReadWriteTarget::ObjectAll(self.object_id()), None);
    }

    let value = analyzer
      .factory
      .try_union(allocator::Vec::from_iter_in(context.values.iter().copied(), analyzer.allocator))
      .unwrap_or(analyzer.factory.undefined);
    if mangable {
      analyzer.factory.computed(value, (context.extra_deps, dep))
    } else {
      analyzer.factory.computed(value, (context.extra_deps, dep, key))
    }
  }

  fn get_keyed(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    context: &mut GetPropertyContext<'a>,
    key: PropertyKeyValue<'a>,
    mut key_atom: Option<MangleAtom>,
    exhaustive_deps: Option<&mut Vec<PropertyKeyValue<'a>>>,
  ) -> bool {
    if self.is_mangable() {
      if key_atom.is_none() {
        self.disable_mangling(analyzer);
      }
    } else {
      key_atom = None;
    }

    let mut string_keyed = self.keyed.borrow_mut();
    if let Some(property) = string_keyed.get_mut(&key) {
      if let Some(exhaustive_deps) = exhaustive_deps
        && property.may_be_unconsumed_field()
      {
        exhaustive_deps.push(key);
      }
      property.get(analyzer, context, key_atom);
      if property.definite {
        return true;
      }
    } else if let Some(exhaustive_deps) = exhaustive_deps {
      exhaustive_deps.push(key);
    }

    match self.prototype.get() {
      ObjectPrototype::ImplicitOrNull => false,
      ObjectPrototype::Builtin(prototype) => {
        if let Some(value) = prototype.get_keyed(analyzer, key, context.this) {
          context.values.push(if let Some(key_atom) = key_atom {
            analyzer.factory.computed(value, key_atom)
          } else {
            value
          });
          true
        } else {
          false
        }
      }
      ObjectPrototype::Custom(prototype) => {
        prototype.get_keyed(analyzer, context, key, key_atom, None)
      }
      ObjectPrototype::Unknown(_unknown) => false,
    }
  }

  fn get_any_keyed(&self, analyzer: &Analyzer<'a>, context: &mut GetPropertyContext<'a>) {
    for property in self.keyed.borrow_mut().values_mut() {
      property.get(analyzer, context, None);
    }
    match self.prototype.get() {
      ObjectPrototype::ImplicitOrNull => {}
      ObjectPrototype::Builtin(_prototype) => {
        // TODO: Control via an option
      }
      ObjectPrototype::Custom(prototype) => prototype.get_any_keyed(analyzer, context),
      ObjectPrototype::Unknown(_dep) => {}
    }
  }

  fn get_unknown_keyed(
    &self,
    analyzer: &Analyzer<'a>,
    context: &mut GetPropertyContext<'a>,
  ) -> bool {
    let mut unknown_keyed = self.unknown.borrow_mut();
    unknown_keyed.get(analyzer, context, None);
    (match self.prototype.get() {
      ObjectPrototype::ImplicitOrNull => false,
      ObjectPrototype::Builtin(_) => false,
      ObjectPrototype::Custom(prototype) => prototype.get_unknown_keyed(analyzer, context),
      ObjectPrototype::Unknown(dep) => {
        context.values.push(analyzer.factory.computed_unknown(dep));
        true
      }
    }) || !unknown_keyed.possible_values.is_empty()
  }
}
