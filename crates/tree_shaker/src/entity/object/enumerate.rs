use super::{get::GetPropertyContext, ObjectEntity};
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
    let mut context = GetPropertyContext {
      key: analyzer.factory.never,
      values: vec![],
      getters: vec![],
      extra_deps: vec![],
    };

    {
      {
        let mut unknown_keyed = self.unknown_keyed.borrow_mut();
        unknown_keyed.get(analyzer, &mut context, None);
        if let Some(rest) = &mut *self.rest.borrow_mut() {
          rest.get(analyzer, &mut context, None);
        }
      }

      for getter in context.getters.drain(..) {
        context.values.push(getter.call_as_getter(
          analyzer,
          analyzer.factory.empty_consumable,
          self,
        ));
      }

      if let Some(value) = analyzer.factory.try_union(mem::take(&mut context.values)) {
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
        let property = string_keyed.get_mut(&key).unwrap();

        if !property.enumerable {
          continue;
        }

        let definite = property.definite;
        let key_entity = if mangable {
          analyzer.factory.mangable_string(key, property.mangling.unwrap())
        } else {
          analyzer.factory.string(key)
        };

        property.get(analyzer, &mut context, None);
        mem::drop(string_keyed);
        for getter in context.getters.drain(..) {
          context.values.push(getter.call_as_getter(
            analyzer,
            analyzer.factory.empty_consumable,
            self,
          ));
        }

        if let Some(value) = analyzer.factory.try_union(mem::take(&mut context.values)) {
          result.push((definite, key_entity, value));
        }
      }
    }

    analyzer.pop_cf_scope();

    (result, analyzer.consumable((dep, context.extra_deps)))
  }
}
