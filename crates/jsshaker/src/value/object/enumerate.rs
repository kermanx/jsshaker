use std::mem;

use oxc::allocator;
use rustc_hash::FxHashMap;

use super::{ObjectValue, get::GetPropertyContext};
use crate::{
  analyzer::Analyzer,
  analyzer::rw_tracking::ReadWriteTarget,
  dep::Dep,
  scope::CfScopeKind,
  value::{EnumeratedProperties, PropertyKeyValue, consumed_object},
};

impl<'a> ObjectValue<'a> {
  pub fn enumerate_properties(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
  ) -> EnumeratedProperties<'a> {
    if self.is_self_or_proto_consumed() {
      return consumed_object::enumerate_properties(self, analyzer, dep);
    }

    analyzer.push_cf_scope_with_deps(CfScopeKind::Dependent, analyzer.factory.vec1(dep), true);

    let mut context = GetPropertyContext {
      key: analyzer.factory.never,
      values: vec![],
      getters: vec![],
      extra_deps: analyzer.factory.vec(),
    };

    let mut unknown = None;
    {
      {
        let mut unknown_keyed = self.unknown.borrow_mut();
        unknown_keyed.get(analyzer, &mut context, None);
        if let Some(rest) = &self.rest {
          rest.borrow_mut().get(analyzer, &mut context, None);
        }
      }

      for getter in context.getters.drain(..) {
        context.values.push(getter.call_as_getter(analyzer, analyzer.factory.no_dep, self.into()));
      }

      if let Some(value) = analyzer
        .factory
        .try_union(allocator::Vec::from_iter_in(context.values.drain(..), analyzer.allocator))
      {
        unknown = Some(value);
      }
    }

    let mut known = FxHashMap::default();
    {
      let string_keyed = self.keyed.borrow();
      let keys = string_keyed.keys().cloned().collect::<Vec<_>>();
      mem::drop(string_keyed);
      let mangable = self.is_mangable();
      for key in keys {
        let mut string_keyed = self.keyed.borrow_mut();
        let property = string_keyed.get_mut(&key).unwrap();

        if !property.enumerable {
          continue;
        }

        let definite = property.definite;
        let (key, key_entity) = if let PropertyKeyValue::String(key) = key {
          (
            PropertyKeyValue::String(key),
            analyzer.factory.string(key, mangable.then(|| property.mangling.unwrap())),
          )
        } else {
          todo!()
        };

        property.get(analyzer, &mut context, None);
        mem::drop(string_keyed);
        for getter in context.getters.drain(..) {
          context.values.push(getter.call_as_getter(
            analyzer,
            analyzer.factory.no_dep,
            self.into(),
          ));
        }

        if let Some(value) = analyzer
          .factory
          .try_union(allocator::Vec::from_iter_in(context.values.drain(..), analyzer.allocator))
        {
          known.insert(key, (definite, key_entity, value));
        }
      }
    }

    analyzer.pop_cf_scope();

    analyzer.track_read(self.cf_scope, ReadWriteTarget::ObjectAll(self.object_id), None);

    EnumeratedProperties { known, unknown, dep: analyzer.dep((dep, context.extra_deps)) }
  }
}
