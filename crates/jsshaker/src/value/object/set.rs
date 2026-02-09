use super::{ObjectProperty, ObjectPropertyValue, ObjectPrototype, ObjectValue};
use crate::{
  analyzer::{Analyzer, rw_tracking::ReadWriteTarget},
  dep::{Dep, DepCollector, DepVec},
  entity::Entity,
  mangling::MangleAtom,
  scope::CfScopeKind,
  utils::Found,
  value::{PropertyKeyValue, ValueTrait, escaped},
};

pub struct PendingSetter<'a> {
  pub non_det: bool,
  pub dep: Option<Dep<'a>>,
  pub setter: Entity<'a>,
}

impl<'a> ObjectValue<'a> {
  pub fn set_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    key: Entity<'a>,
    value: Entity<'a>,
  ) {
    if self.immutable {
      return escaped::set_property(analyzer, dep, key, value);
    }

    if self.is_self_or_proto_included() {
      analyzer.include(self);
      return escaped::set_property(analyzer, dep, key, value);
    }

    let (target_depth, is_exhaustive, mut non_det, deps) = self.prepare_mutation(analyzer, dep);
    let key_literals = key.get_literals(analyzer);

    if is_exhaustive && key_literals.is_none() {
      self.include(analyzer);
      return escaped::set_property(analyzer, dep, key, value);
    }

    let value = analyzer.factory.computed(value, deps);
    let non_mangable_value = analyzer.factory.computed(value, key);

    let mut setters = vec![];
    let mut deferred_deps = vec![];

    if self.lookup_unknown_keyed_setters(analyzer, &mut setters).may_found() {
      non_det = true;
    }

    if let Some(key_literals) = key_literals {
      let mut keyed = self.keyed.borrow_mut();

      non_det |= key_literals.len() > 1;

      let mangable = self.check_mangable(analyzer, &key_literals);
      let value = if mangable { value } else { non_mangable_value };

      for &key_literal in &key_literals {
        let (key_str, key_atom) = key_literal.into();
        let value = if key_atom.is_none() { non_mangable_value } else { value };

        if key_str.is_proto() {
          drop(keyed);
          return self.set_prototype_from_value(analyzer, non_det, dep, key, value);
        }

        let exists = if let Some(property) = keyed.get_mut(&key_str) {
          if property.set(
            analyzer,
            is_exhaustive,
            key_str.is_private_identifier(),
            non_det,
            key,
            if mangable { key_atom } else { None },
            value,
            &mut setters,
            &mut deferred_deps,
          ) {
            analyzer.track_write(
              target_depth,
              ReadWriteTarget::ObjectField(self.object_id(), key_literal.into()),
              None,
            );
            analyzer.request_exhaustive_callbacks(ReadWriteTarget::ObjectField(
              self.object_id(),
              key_str,
            ));
          }
          if property.definite {
            continue;
          }
          true
        } else {
          analyzer
            .request_exhaustive_callbacks(ReadWriteTarget::ObjectField(self.object_id(), key_str));
          false
        };

        if let Some(rest) = &self.rest {
          rest.borrow_mut().set(
            analyzer,
            false,
            false,
            true,
            key,
            None,
            value,
            &mut setters,
            &mut deferred_deps,
          );
          continue;
        }

        let found =
          self.lookup_keyed_setters_on_proto(analyzer, key, key_str, key_atom, &mut setters);
        if exists || found.must_found() {
          continue;
        }

        if mangable && let Some(key_atom) = key_atom {
          self.add_to_mangling_group(analyzer, key_atom);
        }
        keyed.insert(
          key_str,
          ObjectProperty {
            definite: !non_det && found.must_not_found(),
            enumerable: true, /* TODO: Object.defineProperty */
            possible_values: analyzer.factory.vec1(if is_exhaustive {
              ObjectPropertyValue::new_included(analyzer, analyzer.factory.vec1(value))
            } else {
              ObjectPropertyValue::Field(value, false)
            }),
            non_existent: DepCollector::new(analyzer.factory.vec()),
            key: Some(key),
            mangling: if mangable { key_atom } else { None },
          },
        );
      }
    } else {
      if is_exhaustive {
        analyzer.track_write(target_depth, ReadWriteTarget::ObjectAll(self.object_id()), None);
      }
      analyzer.request_exhaustive_callbacks(ReadWriteTarget::ObjectAll(self.object_id()));

      self.disable_mangling(analyzer);

      non_det = true;

      let mut unknown_keyed = self.unknown.borrow_mut();
      unknown_keyed.possible_values.push(ObjectPropertyValue::Field(non_mangable_value, false));

      let mut string_keyed = self.keyed.borrow_mut();
      for property in string_keyed.values_mut() {
        property.lookup_setters(analyzer, None, &mut setters);
      }

      if let Some(rest) = &self.rest {
        rest.borrow_mut().lookup_setters(analyzer, None, &mut setters);
      }

      self.lookup_any_string_keyed_setters_on_proto(analyzer, &mut setters);
    }

    if !setters.is_empty() {
      let non_det = non_det || setters.len() > 1 || setters[0].non_det;
      analyzer.push_cf_scope_with_deps(
        CfScopeKind::Dependent,
        analyzer.factory.vec1(analyzer.dep((dep, key))),
        non_det,
      );
      for s in setters {
        analyzer.cf_scope_mut().exited = if non_det { None } else { Some(false) };
        s.setter.call_as_setter(analyzer, s.dep, self.into(), non_mangable_value);
      }
      analyzer.pop_cf_scope();
    }

    analyzer.include(deferred_deps);
  }

  fn lookup_unknown_keyed_setters(
    &self,
    analyzer: &mut Analyzer<'a>,
    setters: &mut Vec<PendingSetter<'a>>,
  ) -> Found {
    let mut found = Found::False;

    found += self.unknown.borrow_mut().lookup_setters(analyzer, None, setters);

    match self.prototype.get() {
      ObjectPrototype::ImplicitOrNull => {}
      ObjectPrototype::Builtin(_) => {}
      ObjectPrototype::Custom(prototype) => {
        found += prototype.lookup_unknown_keyed_setters(analyzer, setters);
      }
      ObjectPrototype::Unknown(dep) => {
        setters.push(PendingSetter {
          non_det: true,
          dep: Some(dep),
          setter: analyzer.factory.computed_unknown(dep),
        });
        found = Found::Unknown;
      }
    }

    found
  }

  fn lookup_keyed_setters_on_proto(
    &self,
    analyzer: &mut Analyzer<'a>,
    key: Entity<'a>,
    key_str: PropertyKeyValue<'a>,
    mut key_atom: Option<MangleAtom>,
    setters: &mut Vec<PendingSetter<'a>>,
  ) -> Found {
    match self.prototype.get() {
      ObjectPrototype::ImplicitOrNull => Found::False,
      ObjectPrototype::Builtin(_) => Found::False, // FIXME: Setters on builtin prototypes
      ObjectPrototype::Custom(prototype) => {
        let found1 = if let Some(property) = prototype.keyed.borrow_mut().get_mut(&key_str) {
          if prototype.is_mangable() {
            if key_atom.is_none() {
              prototype.disable_mangling(analyzer);
            }
          } else {
            key_atom = None;
          }
          let mangling_dep = key_atom.map(|atom| property.make_mangling_dep(key, atom));
          let found = property.lookup_setters(analyzer, mangling_dep, setters);
          if property.definite && found.must_found() {
            return Found::True;
          }
          if found == Found::False { Found::False } else { Found::Unknown }
        } else {
          Found::False
        };

        let found2 =
          prototype.lookup_keyed_setters_on_proto(analyzer, key, key_str, key_atom, setters);

        found1 + found2
      }
      ObjectPrototype::Unknown(_dep) => Found::Unknown,
    }
  }

  fn lookup_any_string_keyed_setters_on_proto(
    &self,
    analyzer: &mut Analyzer<'a>,
    setters: &mut Vec<PendingSetter<'a>>,
  ) {
    match self.prototype.get() {
      ObjectPrototype::ImplicitOrNull => {}
      ObjectPrototype::Builtin(_) => {}
      ObjectPrototype::Custom(prototype) => {
        if prototype.is_mangable() {
          prototype.disable_mangling(analyzer);
        }

        for property in prototype.keyed.borrow_mut().values_mut() {
          property.lookup_setters(analyzer, None, setters);
        }

        prototype.lookup_any_string_keyed_setters_on_proto(analyzer, setters);
      }
      ObjectPrototype::Unknown(_dep) => {}
    }
  }

  pub(super) fn prepare_mutation(
    &self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
  ) -> (usize, bool, bool, DepVec<'a>) {
    let target_depth = analyzer.find_first_different_cf_scope(self.cf_scope);
    let mut exhaustive = false;
    let mut non_det = false;
    let mut deps = analyzer.factory.vec1(dep);
    for depth in target_depth..analyzer.scoping.cf.stack_len() {
      let scope = analyzer.scoping.cf.data_at_mut(depth);
      exhaustive |= scope.is_exhaustive();
      non_det |= scope.non_det();
      if let Some(dep) = scope.deps.collect(analyzer.factory) {
        deps.push(dep);
      }
    }
    (target_depth, exhaustive, non_det, deps)
  }

  pub fn set_prototype_from_value(
    &self,
    analyzer: &mut Analyzer<'a>,
    non_det: bool,
    dep: Dep<'a>,
    key: Entity<'a>,
    value: Entity<'a>,
  ) {
    if analyzer.config.precise_dynamic_prototype {
      self.set_prototype(ObjectPrototype::Unknown(analyzer.factory.no_dep));
      return escaped::set_property(analyzer, dep, key, value);
    }

    let is_nullish = value.test_nullish();
    if non_det {
      let old_proto = match self.prototype.get() {
        ObjectPrototype::ImplicitOrNull => {
          if is_nullish == Some(true) {
            self.add_extra_dep(analyzer.dep((dep, key, value)));
            return;
          }
          None
        }
        ObjectPrototype::Builtin(_) => None,
        ObjectPrototype::Custom(prototype) => Some(analyzer.dep(prototype)),
        ObjectPrototype::Unknown(dep) => Some(dep),
      };
      self.set_prototype(ObjectPrototype::Unknown(analyzer.dep((old_proto, dep, key, value))));
    } else if is_nullish == Some(true) {
      self.set_prototype(ObjectPrototype::ImplicitOrNull);
      self.add_extra_dep(analyzer.dep((dep, key, value)));
    } else if let Some(object) = value.as_object() {
      self.set_prototype(ObjectPrototype::Custom(object));
      self.add_extra_dep(analyzer.dep((dep, key, value.get_shallow_dep(analyzer))));
    } else {
      self.set_prototype(ObjectPrototype::Unknown(analyzer.dep((dep, key, value))));
    }
  }
}
