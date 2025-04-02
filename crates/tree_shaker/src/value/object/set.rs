use super::{ObjectProperty, ObjectPropertyKey, ObjectPropertyValue, ObjectPrototype, ObjectValue};
use crate::{
  analyzer::{Analyzer, exhaustive::ExhaustiveDepId},
  dep::{Dep, DepCollector, DepVec},
  entity::Entity,
  mangling::{MangleAtom, MangleConstraint},
  scope::CfScopeKind,
  utils::Found,
  value::consumed_object,
};

pub struct PendingSetter<'a> {
  pub indeterminate: bool,
  pub dep: Dep<'a>,
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
    let value = analyzer.factory.computed(value, deps);
    let non_mangable_value = analyzer.factory.computed(value, key);

    let mut setters = vec![];

    if self.lookup_unknown_keyed_setters(analyzer, &mut setters).may_found() {
      indeterminate = true;
    }

    if let Some(key_literals) = key.get_to_literals(analyzer) {
      for &key_literal in &key_literals {
        analyzer.mark_exhaustive_write(
          ExhaustiveDepId::ObjectField(self.object_id, key_literal.into()),
          target_depth,
        );
      }

      let mut keyed = self.keyed.borrow_mut();
      let mut rest = self.rest.borrow_mut();

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
          property.set(analyzer, indeterminate, value, &mut setters);
          let was_consumed = property.consumed;
          property.consumed |= is_exhaustive;
          if !was_consumed {
            analyzer
              .request_exhaustive_callbacks(ExhaustiveDepId::ObjectField(self.object_id, key_str));
          }
          if property.definite {
            continue;
          }
        } else {
          analyzer
            .request_exhaustive_callbacks(ExhaustiveDepId::ObjectField(self.object_id, key_str));
        }

        if let Some(rest) = &mut *rest {
          rest.set(analyzer, true, value, &mut setters);
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
            consumed: is_exhaustive,
            definite: !indeterminate && found.must_not_found(),
            enumerable: true, /* TODO: Object.defineProperty */
            possible_values: analyzer.factory.vec1(ObjectPropertyValue::Field(value, false)),
            non_existent: DepCollector::new(analyzer.factory.vec()),
            key: Some(key),
            mangling: mangable.then(|| key_atom.unwrap()),
          },
        );
      }
    } else {
      if is_exhaustive {
        analyzer.mark_exhaustive_write(ExhaustiveDepId::ObjectAll(self.object_id), target_depth);
      }
      analyzer.request_exhaustive_callbacks(ExhaustiveDepId::ObjectAll(self.object_id));

      self.disable_mangling(analyzer);

      indeterminate = true;

      let mut unknown_keyed = self.unknown.borrow_mut();
      unknown_keyed.possible_values.push(ObjectPropertyValue::Field(non_mangable_value, false));

      let mut string_keyed = self.keyed.borrow_mut();
      for property in string_keyed.values_mut() {
        property.set(analyzer, true, non_mangable_value, &mut setters);
        property.consumed |= is_exhaustive;
      }

      if let Some(rest) = &mut *self.rest.borrow_mut() {
        rest.set(analyzer, true, non_mangable_value, &mut setters);
      }

      self.lookup_any_string_keyed_setters_on_proto(analyzer, &mut setters);
    }

    if !setters.is_empty() {
      let indeterminate = indeterminate || setters.len() > 1 || setters[0].indeterminate;
      analyzer.push_cf_scope_with_deps(
        CfScopeKind::Dependent,
        analyzer.factory.vec1(analyzer.dep((dep, key))),
        if indeterminate { None } else { Some(false) },
      );
      for s in setters {
        s.setter.call_as_setter(analyzer, s.dep, self.into(), non_mangable_value);
      }
      analyzer.pop_cf_scope();
    }
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
          dep,
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
    key_str: ObjectPropertyKey<'a>,
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
      if let Some(dep) = scope.deps.try_collect(analyzer.factory) {
        deps.push(dep);
      }
    }
    (target_depth, exhaustive, indeterminate, deps)
  }
}
