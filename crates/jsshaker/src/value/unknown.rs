use std::marker::PhantomData;

use super::{
  ArgumentsValue, EnumeratedProperties, IteratedElements, TypeofResult, UnionHint, ValueTrait,
  cacheable::Cacheable, escaped,
};
use crate::{analyzer::Analyzer, dep::Dep, entity::Entity};

#[derive(Debug, Default)]
pub struct UnknownValue<'a>(PhantomData<&'a ()>);

impl<'a> ValueTrait<'a> for UnknownValue<'a> {
  fn include(&'a self, _analyzer: &mut Analyzer<'a>) {}

  fn unknown_mutate(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) {
    escaped::unknown_mutate(analyzer, dep)
  }

  fn get_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    key: Entity<'a>,
  ) -> Entity<'a> {
    escaped::get_property(self, analyzer, dep, key)
  }

  fn set_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    key: Entity<'a>,
    value: Entity<'a>,
  ) {
    self.include(analyzer);
    escaped::set_property(analyzer, dep, key, value)
  }

  fn enumerate_properties(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
  ) -> EnumeratedProperties<'a> {
    if analyzer.config.unknown_property_read_side_effects {
      self.include(analyzer);
    }
    escaped::enumerate_properties(self, analyzer, dep)
  }

  fn delete_property(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>, key: Entity<'a>) {
    self.include(analyzer);
    escaped::delete_property(analyzer, dep, key)
  }

  fn call(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    this: Entity<'a>,
    args: ArgumentsValue<'a>,
  ) -> Entity<'a> {
    escaped::call(self, analyzer, dep, this, args)
  }

  fn construct(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    args: ArgumentsValue<'a>,
  ) -> Entity<'a> {
    escaped::construct(self, analyzer, dep, args)
  }

  fn jsx(&'a self, analyzer: &mut Analyzer<'a>, props: Entity<'a>) -> Entity<'a> {
    escaped::jsx(self, analyzer, props)
  }

  fn r#await(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) -> Entity<'a> {
    self.include(analyzer);
    escaped::r#await(analyzer, dep)
  }

  fn iterate(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) -> IteratedElements<'a> {
    self.include(analyzer);
    escaped::iterate(analyzer, dep)
  }

  fn coerce_string(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.computed_unknown_string(self)
  }

  fn coerce_number(&'a self, _analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.into()
  }

  fn coerce_primitive(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.unknown
  }

  fn coerce_property_key(&'a self, _analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.into()
  }

  fn coerce_jsx_child(&'a self, _analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.into()
  }

  fn test_typeof(&self) -> TypeofResult {
    TypeofResult::_Unknown
  }

  fn test_truthy(&self) -> Option<bool> {
    None
  }

  fn test_nullish(&self) -> Option<bool> {
    None
  }

  fn is_shared_value(&self) -> bool {
    true
  }
  fn get_union_hint(&self) -> UnionHint {
    UnionHint::Unknown
  }

  fn as_cacheable(&self, _analyzer: &Analyzer<'a>) -> Option<Cacheable<'a>> {
    Some(Cacheable::Unknown)
  }
}

impl UnknownValue<'_> {
  pub fn new() -> Self {
    Self::default()
  }
}
