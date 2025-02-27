use super::ObjectEntity;
use crate::{
  analyzer::Analyzer,
  consumable::Consumable,
  entity::{consumed_object, EnumeratedProperties},
  scope::CfScopeKind,
};
use std::mem;

impl<'a> ObjectEntity<'a> {
  pub fn enumerate_properties(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
  ) -> EnumeratedProperties<'a> {
    if self.consumed.get() {
      return consumed_object::enumerate_properties(self, analyzer, dep);
    }

    analyzer.mark_object_property_exhaustive_read(self.cf_scope, self.object_id);
    analyzer.push_cf_scope_with_deps(CfScopeKind::Dependent, vec![dep], None);

    let mut result = vec![];
    let mut non_existent = vec![];

    {
      let mut values = vec![];
      let mut getters = vec![];

      {
        let mut unknown_keyed = self.unknown_keyed.borrow_mut();
        unknown_keyed.get(analyzer, &mut values, &mut getters, &mut non_existent);
        if let Some(rest) = &mut *self.rest.borrow_mut() {
          rest.get(analyzer, &mut values, &mut getters, &mut non_existent);
        }
      }

      for getter in getters {
        values.push(getter.call_as_getter(analyzer, analyzer.factory.empty_consumable, self));
      }

      if let Some(value) = analyzer.factory.try_union(values) {
        result.push((false, analyzer.factory.unknown_primitive, value));
      }
    }

    {
      let string_keyed = self.string_keyed.borrow();
      let keys = string_keyed.keys().cloned().collect::<Vec<_>>();
      mem::drop(string_keyed);
      let mangable = self.is_mangable();
      for key in keys {
        let mut string_keyed = self.string_keyed.borrow_mut();
        let properties = string_keyed.get_mut(&key).unwrap();

        let definite = properties.definite;
        let key_entity = if mangable {
          analyzer.factory.mangable_string(key, properties.mangling.unwrap())
        } else {
          analyzer.factory.string(key)
        };

        let mut values = vec![];
        let mut getters = vec![];
        properties.get(analyzer, &mut values, &mut getters, &mut non_existent);
        mem::drop(string_keyed);
        for getter in getters {
          values.push(getter.call_as_getter(analyzer, analyzer.factory.empty_consumable, self));
        }

        if let Some(value) = analyzer.factory.try_union(values) {
          result.push((definite, key_entity, value));
        }
      }
    }

    analyzer.pop_cf_scope();

    (result, analyzer.consumable((dep, non_existent)))
  }
}
