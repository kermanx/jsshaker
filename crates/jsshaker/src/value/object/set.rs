use super::{ObjectProperty, ObjectPropertyValue, ObjectPrototype, ObjectValue};
use crate::{
  analyzer::{Analyzer, rw_tracking::ReadWriteTarget},
  dep::{Dep, DepCollector, DepVec},
  entity::Entity,
  mangling::{MangleAtom, MangleConstraint},
  scope::CfScopeKind,
  utils::Found,
  value::{PropertyKeyValue, ValueTrait, consumed_object},
};

pub struct PendingSetter<'a> {
  pub indeterminate: bool,
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
    if self.consumed.get() {
      return consumed_object::set_property(analyzer, dep, key, value);
    }

    let (target_depth, is_exhaustive, mut indeterminate, deps) =
      self.prepare_mutation(analyzer, dep);
    let key_literals = key.get_to_literals(analyzer);

    if is_exhaustive && key_literals.is_none() {
      self.consume(analyzer);
      return consumed_object::set_property(analyzer, dep, key, value);
    }

    let value = analyzer.factory.computed(value, deps);
    let non_mangable_value = analyzer.factory.computed(value, key);

    let mut setters = vec![];
    let mut deferred_deps = vec![];

    if self.lookup_unknown_keyed_setters(analyzer, &mut setters).may_found() {
      indeterminate = true;
    }

    if let Some(key_literals) = key_literals {
      let mut keyed = self.keyed.borrow_mut();

      indeterminate |= key_literals.len() > 1;

      let mangable = self.check_mangable(analyzer, &key_literals);
      let value = if mangable { value } else { non_mangable_value };

      for key_literal in key_literals {
        let (key_str, key_atom) = key_literal.into();
        if let Some(property) = keyed.get_mut(&key_str) {
          let value = if mangable {
            let prev_key = property.key.unwrap();
            let prev_atom = property.mangling.unwrap();
            analyzer.factory.mangable(
              value,
              (prev_key, key),
              MangleConstraint::Eq(prev_atom, key_atom.unwrap()),
            )
          } else {
            value
          };
          if property.set(
            analyzer,
            is_exhaustive,
            indeterminate,
            value,
            &mut setters,
            &mut deferred_deps,
          ) {
            analyzer.track_write(
              target_depth,
              ReadWriteTarget::ObjectField(self.object_id, key_literal.into()),
              None,
            );
            analyzer
              .request_exhaustive_callbacks(ReadWriteTarget::ObjectField(self.object_id, key_str));
          }
          if property.definite {
            continue;
          }
        } else {
          analyzer
            .request_exhaustive_callbacks(ReadWriteTarget::ObjectField(self.object_id, key_str));
        }

        if let Some(rest) = &self.rest {
          rest.borrow_mut().set(analyzer, false, true, value, &mut setters, &mut deferred_deps);
          continue;
        }

        let found = self.lookup_keyed_setters_on_proto(analyzer, key_str, key_atom, &mut setters);
        if found.must_found() {
          continue;
        }

        if mangable {
          self.add_to_mangling_group(analyzer, key_atom.unwrap());
        }
        keyed.insert(
          key_str,
          ObjectProperty {
            definite: !indeterminate && found.must_not_found(),
            enumerable: true, /* TODO: Object.defineProperty */
            possible_values: analyzer.factory.vec1(if is_exhaustive {
              ObjectPropertyValue::new_consumed(analyzer, analyzer.factory.vec1(value))
            } else {
              ObjectPropertyValue::Field(value, false)
            }),
            non_existent: DepCollector::new(analyzer.factory.vec()),
            key: Some(key),
            mangling: mangable.then(|| key_atom.unwrap()),
          },
        );
      }
    } else {
      if is_exhaustive {
        analyzer.track_write(target_depth, ReadWriteTarget::ObjectAll(self.object_id), None);
      }
      analyzer.request_exhaustive_callbacks(ReadWriteTarget::ObjectAll(self.object_id));

      self.disable_mangling(analyzer);

      indeterminate = true;

      let mut unknown_keyed = self.unknown.borrow_mut();
      unknown_keyed.possible_values.push(ObjectPropertyValue::Field(non_mangable_value, false));

      let mut string_keyed = self.keyed.borrow_mut();
      for property in string_keyed.values_mut() {
        property.lookup_setters(analyzer, &mut setters);
      }

      if let Some(rest) = &self.rest {
        rest.borrow_mut().lookup_setters(analyzer, &mut setters);
      }

      self.lookup_any_string_keyed_setters_on_proto(analyzer, &mut setters);
    }

    if !setters.is_empty() {
      let indeterminate = indeterminate || setters.len() > 1 || setters[0].indeterminate;
      analyzer.push_cf_scope_with_deps(
        CfScopeKind::Dependent,
        analyzer.factory.vec1(analyzer.dep((dep, key))),
        indeterminate,
      );
      for s in setters {
        analyzer.cf_scope_mut().exited = if indeterminate { None } else { Some(false) };
        s.setter.call_as_setter(analyzer, s.dep, self.into(), non_mangable_value);
      }
      analyzer.pop_cf_scope();
    }

    analyzer.consume(deferred_deps);
  }

  fn lookup_unknown_keyed_setters(
    &self,
    analyzer: &mut Analyzer<'a>,
    setters: &mut Vec<PendingSetter<'a>>,
  ) -> Found {
    let mut found = Found::False;

    found += self.unknown.borrow_mut().lookup_setters(analyzer, setters);

    match self.prototype.get() {
      ObjectPrototype::ImplicitOrNull => {}
      ObjectPrototype::Builtin(_) => {}
      ObjectPrototype::Custom(prototype) => {
        found += prototype.lookup_unknown_keyed_setters(analyzer, setters);
      }
      ObjectPrototype::Unknown(dep) => {
        setters.push(PendingSetter {
          indeterminate: true,
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
          let found = property.lookup_setters(analyzer, setters);
          if property.definite && found.must_found() {
            return Found::True;
          }
          if found == Found::False { Found::False } else { Found::Unknown }
        } else {
          Found::False
        };

        let found2 = prototype.lookup_keyed_setters_on_proto(analyzer, key_str, key_atom, setters);

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
          property.lookup_setters(analyzer, setters);
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
    let mut indeterminate = false;
    let mut deps = analyzer.factory.vec1(dep);
    for depth in target_depth..analyzer.scoping.cf.stack.len() {
      let scope = analyzer.scoping.cf.get_mut_from_depth(depth);
      exhaustive |= scope.is_exhaustive();
      indeterminate |= scope.is_indeterminate();
      if let Some(dep) = scope.deps.collect(analyzer.factory) {
        deps.push(dep);
      }
    }
    (target_depth, exhaustive, indeterminate, deps)
  }
}
