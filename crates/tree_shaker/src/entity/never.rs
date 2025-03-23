use crate::{analyzer::Analyzer, dep::Dep};

use super::{Entity, ValueTrait};

#[derive(Debug, Clone, Copy)]
pub struct NeverEntity;

impl<'a> ValueTrait<'a> for NeverEntity {
  fn consume(&'a self, _analyzer: &mut Analyzer<'a>) {}

  fn unknown_mutate(&'a self, _analyzer: &mut Analyzer<'a>, _dep: Dep<'a>) {}

  fn get_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    _dep: Dep<'a>,
    _key: Entity<'a>,
  ) -> Entity<'a> {
    analyzer.factory.never
  }
  fn set_property(
    &'a self,
    _analyzer: &mut Analyzer<'a>,
    _dep: Dep<'a>,
    _key: Entity<'a>,
    _value: Entity<'a>,
  ) {
  }
  fn enumerate_properties(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    _dep: Dep<'a>,
  ) -> super::EnumeratedProperties<'a> {
    (Vec::new(), analyzer.factory.no_dep)
  }
  fn delete_property(&'a self, _analyzer: &mut Analyzer<'a>, _dep: Dep<'a>, _key: Entity<'a>) {}
  fn call(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    _dep: Dep<'a>,
    _this: Entity<'a>,
    _args: Entity<'a>,
  ) -> Entity<'a> {
    analyzer.factory.never
  }
  fn construct(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    _dep: Dep<'a>,
    _args: Entity<'a>,
  ) -> Entity<'a> {
    analyzer.factory.never
  }
  fn jsx(&'a self, analyzer: &mut Analyzer<'a>, _props: Entity<'a>) -> Entity<'a> {
    analyzer.factory.never
  }
  fn r#await(&'a self, analyzer: &mut Analyzer<'a>, _dep: Dep<'a>) -> Entity<'a> {
    analyzer.factory.never
  }
  fn iterate(&'a self, analyzer: &mut Analyzer<'a>, _dep: Dep<'a>) -> super::IteratedElements<'a> {
    (Vec::new(), None, analyzer.factory.no_dep)
  }

  fn get_typeof(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.never
  }
  fn get_to_string(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.never
  }
  fn get_to_numeric(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.never
  }
  fn get_to_boolean(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.never
  }
  fn get_to_property_key(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.never
  }
  fn get_to_jsx_child(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.never
  }
  fn get_to_literals(
    &'a self,
    _analyzer: &Analyzer<'a>,
  ) -> Option<rustc_hash::FxHashSet<super::LiteralEntity<'a>>> {
    Some(rustc_hash::FxHashSet::default())
  }
  fn get_literal(&'a self, _analyzer: &Analyzer<'a>) -> Option<super::LiteralEntity<'a>> {
    None
  }
  fn get_own_keys(&'a self, _analyzer: &Analyzer<'a>) -> Option<Vec<(bool, Entity<'a>)>> {
    Some(vec![])
  }

  fn test_typeof(&self) -> super::TypeofResult {
    super::TypeofResult::_None
  }
  fn test_truthy(&self) -> Option<bool> {
    Some(false)
  }
  fn test_nullish(&self) -> Option<bool> {
    Some(false)
  }
}
