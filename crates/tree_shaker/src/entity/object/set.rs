use super::{ObjectEntity, ObjectProperty, ObjectPropertyValue};
use crate::{
  analyzer::Analyzer,
  consumable::{Consumable, ConsumableCollector},
  entity::{consumed_object, Entity, EntityTrait, LiteralEntity},
  mangling::MangleConstraint,
  scope::CfScopeKind,
};

impl<'a> ObjectEntity<'a> {
  pub fn set_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
    key: Entity<'a>,
    value: Entity<'a>,
  ) {
    if self.consumed.get() {
      return consumed_object::set_property(analyzer, dep, key, value);
    }

    let (has_exhaustive, mut indeterminate, exec_deps) =
      analyzer.pre_mutate_object(self.cf_scope, self.object_id);

    if has_exhaustive {
      self.consume(analyzer);
      return consumed_object::set_property(analyzer, dep, key, value);
    }

    let mut setters = vec![];

    {
      let unknown_keyed = self.unknown_keyed.borrow();
      for possible_value in &unknown_keyed.possible_values {
        if let ObjectPropertyValue::Property(_, setter) = possible_value {
          if let Some(setter) = setter {
            setters.push((true, analyzer.factory.empty_consumable, *setter));
          }
          indeterminate = true;
        }
      }
    }

    let value = analyzer.factory.computed(value, analyzer.consumable((exec_deps, dep)));
    let non_mangable_value = analyzer.factory.computed(value, key);

    if let Some(key_literals) = key.get_to_literals(analyzer) {
      let mut string_keyed = self.string_keyed.borrow_mut();
      let mut rest = self.rest.borrow_mut();

      indeterminate |= key_literals.len() > 1;

      let mangable = self.check_mangable(analyzer, &key_literals);
      let value = if mangable {
        value
      } else {
        self.disable_mangling(analyzer);
        non_mangable_value
      };

      for key_literal in key_literals {
        match key_literal {
          LiteralEntity::String(key_str, key_atom) => {
            if let Some(property) = string_keyed.get_mut(key_str) {
              let value = if mangable {
                let prev_key = property.key.unwrap();
                let prev_atom = property.mangling.unwrap();
                analyzer.factory.mangable(
                  value,
                  (prev_key, key),
                  analyzer.factory.alloc(MangleConstraint::Eq(prev_atom, key_atom.unwrap())),
                )
              } else {
                value
              };
              property.set(analyzer, indeterminate, value, &mut setters);
            } else if let Some(rest) = &mut *rest {
              rest.set(analyzer, true, value, &mut setters);
            } else {
              if mangable {
                self.add_to_mangling_group(analyzer, key_atom.unwrap());
              }
              string_keyed.insert(
                key_str,
                ObjectProperty {
                  definite: !indeterminate,
                  enumerable: true, /* TODO: Object.defineProperty */
                  possible_values: vec![ObjectPropertyValue::Field(value, false)],
                  non_existent: ConsumableCollector::default(),
                  key: Some(key),
                  mangling: mangable.then(|| key_atom.unwrap()),
                },
              );
            }
          }
          LiteralEntity::Symbol(_, _) => todo!(),
          _ => unreachable!("Invalid property key"),
        }
      }
    } else {
      self.disable_mangling(analyzer);

      indeterminate = true;

      let mut unknown_keyed = self.unknown_keyed.borrow_mut();
      unknown_keyed.possible_values.push(ObjectPropertyValue::Field(non_mangable_value, false));

      let mut string_keyed = self.string_keyed.borrow_mut();
      for property in string_keyed.values_mut() {
        property.set(analyzer, true, non_mangable_value, &mut setters);
      }

      if let Some(rest) = &mut *self.rest.borrow_mut() {
        rest.set(analyzer, true, non_mangable_value, &mut setters);
      }
    }

    if !setters.is_empty() {
      let indeterminate = indeterminate || setters.len() > 1 || setters[0].0;
      analyzer.push_cf_scope_with_deps(
        CfScopeKind::Dependent,
        vec![analyzer.consumable((dep, key))],
        if indeterminate { None } else { Some(false) },
      );
      for (_, call_dep, setter) in setters {
        setter.call_as_setter(analyzer, call_dep, self, non_mangable_value);
      }
      analyzer.pop_cf_scope();
    }
  }
}
