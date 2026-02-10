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
  fn include(&'a self, analyzer: &mut Analyzer<'a>) {
    self.value.include(analyzer);
  }

  fn include_mangable(&'a self, analyzer: &mut Analyzer<'a>) -> bool {
    self.value.include_mangable(analyzer)
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

  fn get_shallow_dep(&'a self, analyzer: &Analyzer<'a>) -> Option<Dep<'a>> {
    Some(self.value.get_shallow_dep(analyzer))
  }

  fn coerce_string(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.value.coerce_string(analyzer)
  }

  fn coerce_number(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.value.coerce_number(analyzer)
  }

  fn coerce_primitive(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    let value = self.value.coerce_primitive(analyzer);
    if self.is_coalesce {
      value
    } else {
      analyzer.factory.computed(analyzer.factory.boolean(self.result), value)
    }
  }

  fn coerce_property_key(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.value.coerce_property_key(analyzer)
  }

  fn coerce_jsx_child(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.value.coerce_jsx_child(analyzer)
  }

  fn get_keys(
    &'a self,
    analyzer: &Analyzer<'a>,
    check_proto: bool,
  ) -> Option<Vec<(bool, Entity<'a>)>> {
    self.value.get_keys(analyzer, check_proto)
  }

  fn get_constructor_prototype(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
  ) -> Option<(Dep<'a>, ObjectPrototype<'a>, ObjectPrototype<'a>)> {
    self.value.get_constructor_prototype(analyzer, dep)
  }

  fn as_object(&'a self) -> Option<&'a ObjectValue<'a>> {
    self.value.as_object()
  }

  fn test_typeof(&self) -> TypeofResult {
    self.value.test_typeof()
  }

  fn test_truthy(&self) -> Option<bool> {
    if self.is_coalesce { self.value.test_truthy() } else { Some(self.result) }
  }

  fn test_nullish(&self) -> Option<bool> {
    if self.is_coalesce {
      Some(self.result)
    } else if self.result {
      Some(false) // Truthy value can't be nullish
    } else {
      self.value.test_nullish()
    }
  }

  fn as_cacheable(&self, analyzer: &Analyzer<'a>) -> Option<Cacheable<'a>> {
    self.value.as_cacheable(analyzer)
  }
}

impl<'a> crate::analyzer::Factory<'a> {
  /// Only used when (maybe_left, maybe_right) == (true, true)
  pub fn logical_result(
    &self,
    left: Entity<'a>,
    right: Entity<'a>,
    operator: oxc_syntax::operator::LogicalOperator,
  ) -> Entity<'a> {
    let value = self.union((left, right));
    let result = match operator {
      oxc_syntax::operator::LogicalOperator::Or => match right.test_truthy() {
        Some(true) => true,
        _ => return value,
      },
      oxc_syntax::operator::LogicalOperator::And => match right.test_truthy() {
        Some(false) => false,
        _ => return value,
      },
      oxc_syntax::operator::LogicalOperator::Coalesce => match right.test_nullish() {
        Some(false) => false,
        _ => return value,
      },
    };
    self
      .alloc(LogicalResultValue {
        value,
        is_coalesce: operator == oxc_syntax::operator::LogicalOperator::Coalesce,
        result,
      })
      .into()
  }
}
