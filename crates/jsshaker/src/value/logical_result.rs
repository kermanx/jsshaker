use super::{
  ArgumentsValue, EnumeratedProperties, IteratedElements, ObjectPrototype, TypeofResult,
  ValueTrait, cacheable::Cacheable,
};
use crate::{analyzer::Analyzer, dep::Dep, entity::Entity, value::ObjectValue};

#[derive(Debug, Clone)]
pub struct LogicalResultValue<'a> {
  pub value: Entity<'a>,
  pub is_coalesce: bool,
  pub result: bool,
}

impl<'a> ValueTrait<'a> for LogicalResultValue<'a> {
  fn consume(&'a self, analyzer: &mut Analyzer<'a>) {
    self.value.consume(analyzer);
  }

  fn consume_mangable(&'a self, analyzer: &mut Analyzer<'a>) -> bool {
    self.value.consume_mangable(analyzer)
  }

  fn unknown_mutate(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) {
    self.value.unknown_mutate(analyzer, dep);
  }

  fn get_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    key: Entity<'a>,
  ) -> Entity<'a> {
    self.value.get_property(analyzer, dep, key)
  }

  fn set_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    key: Entity<'a>,
    value: Entity<'a>,
  ) {
    self.value.set_property(analyzer, dep, key, value);
  }

  fn enumerate_properties(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
  ) -> EnumeratedProperties<'a> {
    self.value.enumerate_properties(analyzer, dep)
  }

  fn delete_property(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>, key: Entity<'a>) {
    self.value.delete_property(analyzer, dep, key);
  }

  fn call(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    this: Entity<'a>,
    args: ArgumentsValue<'a>,
  ) -> Entity<'a> {
    self.value.call(analyzer, dep, this, args)
  }

  fn construct(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    args: ArgumentsValue<'a>,
  ) -> Entity<'a> {
    self.value.construct(analyzer, dep, args)
  }

  fn jsx(&'a self, analyzer: &mut Analyzer<'a>, props: Entity<'a>) -> Entity<'a> {
    self.value.jsx(analyzer, props)
  }

  fn r#await(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) -> Entity<'a> {
    self.value.r#await(analyzer, dep)
  }

  fn iterate(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) -> IteratedElements<'a> {
    self.value.iterate(analyzer, dep)
  }

  fn get_shallow_dep(&'a self, analyzer: &Analyzer<'a>) -> Dep<'a> {
    self.value.get_shallow_dep(analyzer)
  }

  fn get_to_string(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.value.get_to_string(analyzer)
  }

  fn get_to_numeric(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.value.get_to_numeric(analyzer)
  }

  fn get_to_boolean(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    let value = self.value.get_to_boolean(analyzer);
    if self.is_coalesce {
      value
    } else {
      analyzer.factory.computed(analyzer.factory.boolean(self.result), value)
    }
  }

  fn get_to_property_key(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.value.get_to_property_key(analyzer)
  }

  fn get_to_jsx_child(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.value.get_to_jsx_child(analyzer)
  }

  fn get_own_keys(&'a self, analyzer: &Analyzer<'a>) -> Option<Vec<(bool, Entity<'a>)>> {
    self.value.get_own_keys(analyzer)
  }

  fn get_constructor_prototype(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
  ) -> Option<(Dep<'a>, ObjectPrototype<'a>, ObjectPrototype<'a>)> {
    self.value.get_constructor_prototype(analyzer, dep)
  }

  fn get_object(&'a self) -> Option<&'a ObjectValue<'a>> {
    self.value.get_object()
  }

  fn test_typeof(&self) -> TypeofResult {
    self.value.test_typeof()
  }

  fn test_truthy(&self) -> Option<bool> {
    if self.is_coalesce { self.value.test_truthy() } else { Some(self.result) }
  }

  fn test_nullish(&self) -> Option<bool> {
    if self.is_coalesce { Some(self.result) } else { self.value.test_nullish() }
  }

  fn as_cacheable(&self, analyzer: &Analyzer<'a>) -> Option<Cacheable<'a>> {
    self.value.as_cacheable(analyzer)
  }
}
