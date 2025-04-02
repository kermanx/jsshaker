use std::cell::Cell;

use oxc::allocator;

use super::{
  EnumeratedProperties, IteratedElements, ObjectPrototype, TypeofResult, ValueTrait,
  consumed_object,
};
use crate::{analyzer::Analyzer, dep::Dep, entity::Entity, use_consumed_flag};

#[derive(Debug)]
pub struct ArgumentsValue<'a> {
  pub consumed: Cell<bool>,
  pub arguments: allocator::Vec<'a, (bool, Entity<'a>)>,
}

impl<'a> ValueTrait<'a> for ArgumentsValue<'a> {
  fn consume(&'a self, analyzer: &mut Analyzer<'a>) {
    use_consumed_flag!(self);

    for (_, entity) in &self.arguments {
      entity.consume(analyzer);
    }
  }

  fn unknown_mutate(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) {
    if self.consumed.get() {
      return consumed_object::unknown_mutate(analyzer, dep);
    }

    for (_, entity) in &self.arguments {
      entity.unknown_mutate(analyzer, dep);
    }
  }

  fn get_property(
    &'a self,
    _analyzer: &mut Analyzer<'a>,
    _dep: Dep<'a>,
    _key: Entity<'a>,
  ) -> Entity<'a> {
    unreachable!()
  }

  fn set_property(
    &'a self,
    _analyzer: &mut Analyzer<'a>,
    _dep: Dep<'a>,
    _key: Entity<'a>,
    _value: Entity<'a>,
  ) {
    unreachable!()
  }

  fn enumerate_properties(
    &'a self,
    _analyzer: &mut Analyzer<'a>,
    _dep: Dep<'a>,
  ) -> EnumeratedProperties<'a> {
    unreachable!()
  }

  fn delete_property(&'a self, _analyzer: &mut Analyzer<'a>, _dep: Dep<'a>, _key: Entity<'a>) {
    unreachable!()
  }

  fn call(
    &'a self,
    _analyzer: &mut Analyzer<'a>,
    _dep: Dep<'a>,
    _this: Entity<'a>,
    _args: Entity<'a>,
  ) -> Entity<'a> {
    unreachable!()
  }

  fn construct(
    &'a self,
    _analyzer: &mut Analyzer<'a>,
    _dep: Dep<'a>,
    _args: Entity<'a>,
  ) -> Entity<'a> {
    unreachable!()
  }

  fn jsx(&'a self, _analyzer: &mut Analyzer<'a>, _props: Entity<'a>) -> Entity<'a> {
    unreachable!()
  }

  fn r#await(&'a self, _analyzer: &mut Analyzer<'a>, _dep: Dep<'a>) -> Entity<'a> {
    unreachable!()
  }

  fn iterate(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) -> IteratedElements<'a> {
    let mut elements = Vec::new();
    let mut rest: Option<allocator::Vec<'a, Entity<'a>>> = None;
    for (spread, entity) in &self.arguments {
      if *spread {
        if let Some(iterated) = entity.iterate_result_union(analyzer, dep) {
          if let Some(rest) = &mut rest {
            rest.push(iterated);
          } else {
            rest = Some(analyzer.factory.vec1(iterated));
          }
        }
      } else if let Some(rest) = &mut rest {
        rest.push(*entity);
      } else {
        elements.push(*entity);
      }
    }
    (elements, rest.map(|val| analyzer.factory.union(val)), dep)
  }

  fn get_typeof(&'a self, _analyzer: &Analyzer<'a>) -> Entity<'a> {
    unreachable!()
  }

  fn get_to_string(&'a self, _analyzer: &Analyzer<'a>) -> Entity<'a> {
    unreachable!()
  }

  fn get_to_numeric(&'a self, _analyzer: &Analyzer<'a>) -> Entity<'a> {
    unreachable!()
  }

  fn get_to_boolean(&'a self, _analyzer: &Analyzer<'a>) -> Entity<'a> {
    unreachable!()
  }

  fn get_to_property_key(&'a self, _analyzer: &Analyzer<'a>) -> Entity<'a> {
    unreachable!()
  }

  fn get_to_jsx_child(&'a self, _analyzer: &Analyzer<'a>) -> Entity<'a> {
    unreachable!()
  }

  fn get_constructor_prototype(
    &'a self,
    _analyzer: &Analyzer<'a>,
    _dep: Dep<'a>,
  ) -> Option<(Dep<'a>, ObjectPrototype<'a>, ObjectPrototype<'a>)> {
    unreachable!()
  }

  fn test_typeof(&self) -> TypeofResult {
    unreachable!()
  }

  fn test_truthy(&self) -> Option<bool> {
    unreachable!()
  }

  fn test_nullish(&self) -> Option<bool> {
    unreachable!()
  }
}
