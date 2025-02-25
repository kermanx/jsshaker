use crate::{analyzer::Analyzer, consumable::Consumable};

use super::{Entity, EntityTrait};

#[derive(Debug, Clone, Copy)]
pub struct NeverEntity;

impl<'a> EntityTrait<'a> for NeverEntity {
  fn consume(&'a self, _analyzer: &mut Analyzer<'a>) {}

  fn unknown_mutate(&'a self, _analyzer: &mut Analyzer<'a>, _dep: Consumable<'a>) {}

  fn get_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    _dep: Consumable<'a>,
    _key: Entity<'a>,
  ) -> Entity<'a> {
    analyzer.factory.never
  }
  fn set_property(
    &'a self,
    _analyzer: &mut Analyzer<'a>,
    _dep: Consumable<'a>,
    _key: Entity<'a>,
    _value: Entity<'a>,
  ) {
  }
  fn enumerate_properties(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    _dep: Consumable<'a>,
  ) -> super::EnumeratedProperties<'a> {
    (Vec::new(), analyzer.factory.empty_consumable)
  }
  fn delete_property(
    &'a self,
    _analyzer: &mut Analyzer<'a>,
    _dep: Consumable<'a>,
    _key: Entity<'a>,
  ) {
  }
  fn call(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    _dep: Consumable<'a>,
    _this: Entity<'a>,
    _args: Entity<'a>,
  ) -> Entity<'a> {
    analyzer.factory.never
  }
  fn construct(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    _dep: Consumable<'a>,
    _args: Entity<'a>,
  ) -> Entity<'a> {
    analyzer.factory.never
  }
  fn jsx(&'a self, analyzer: &mut Analyzer<'a>, _props: Entity<'a>) -> Entity<'a> {
    analyzer.factory.never
  }
  fn r#await(&'a self, analyzer: &mut Analyzer<'a>, _dep: Consumable<'a>) -> Entity<'a> {
    analyzer.factory.never
  }
  fn iterate(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    _dep: Consumable<'a>,
  ) -> super::IteratedElements<'a> {
    (Vec::new(), None, analyzer.factory.empty_consumable)
  }

  fn get_destructable(&'a self, analyzer: &Analyzer<'a>, _dep: Consumable<'a>) -> Consumable<'a> {
    analyzer.factory.empty_consumable
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
    None
  }
  fn get_literal(&'a self, _analyzer: &Analyzer<'a>) -> Option<super::LiteralEntity<'a>> {
    None
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
