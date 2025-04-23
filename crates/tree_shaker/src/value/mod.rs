pub mod arguments;
pub mod array;
pub mod builtin_fn;
mod consumed_object;
mod function;
mod literal;
pub mod logical_result;
pub mod never;
mod object;
pub mod primitive;
pub mod react_element;
mod typeof_result;
pub mod union;
pub mod unknown;
pub mod utils;

pub use literal::LiteralValue;
pub use object::*;
use oxc::{allocator, semantic::SymbolId};
use rustc_hash::{FxHashMap, FxHashSet};
use std::{cmp::Ordering, fmt::Debug};
pub use typeof_result::TypeofResult;
pub use utils::*;

use crate::{
  analyzer::Analyzer,
  dep::{CustomDepTrait, Dep},
  entity::Entity,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PropertyKeyValue<'a> {
  String(&'a str),
  Symbol(SymbolId),
}

pub struct EnumeratedProperties<'a> {
  pub known: FxHashMap<PropertyKeyValue<'a>, (bool, Entity<'a>, Entity<'a>)>,
  pub unknown: Option<Entity<'a>>,
  pub dep: Dep<'a>,
}

/// (vec![known_elements], rest, dep)
pub type IteratedElements<'a> = (Vec<Entity<'a>>, Option<Entity<'a>>, Dep<'a>);

pub trait ValueTrait<'a>: Debug {
  fn consume(&'a self, analyzer: &mut Analyzer<'a>);
  /// Returns true if the entity is completely consumed
  fn consume_mangable(&'a self, analyzer: &mut Analyzer<'a>) -> bool {
    self.consume(analyzer);
    true
  }
  fn unknown_mutate(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>);

  fn get_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    key: Entity<'a>,
  ) -> Entity<'a>;
  fn set_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    key: Entity<'a>,
    value: Entity<'a>,
  );
  fn enumerate_properties(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
  ) -> EnumeratedProperties<'a>;
  fn delete_property(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>, key: Entity<'a>);
  fn call(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    this: Entity<'a>,
    args: Entity<'a>,
  ) -> Entity<'a>;
  fn construct(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>, args: Entity<'a>)
  -> Entity<'a>;
  fn jsx(&'a self, analyzer: &mut Analyzer<'a>, props: Entity<'a>) -> Entity<'a>;
  fn r#await(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) -> Entity<'a>;
  fn iterate(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) -> IteratedElements<'a>;

  fn get_shallow_dep(&'a self, analyzer: &Analyzer<'a>) -> Dep<'a> {
    analyzer.factory.no_dep
  }
  fn get_to_string(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a>;
  fn get_to_numeric(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a>;
  fn get_to_boolean(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a>;
  fn get_to_property_key(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a>;
  fn get_to_jsx_child(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a>;
  fn get_to_literals(&'a self, _analyzer: &Analyzer<'a>) -> Option<FxHashSet<LiteralValue<'a>>> {
    None
  }
  fn get_literal(&'a self, analyzer: &Analyzer<'a>) -> Option<LiteralValue<'a>> {
    self
      .get_to_literals(analyzer)
      .and_then(|set| if set.len() == 1 { set.into_iter().next() } else { None })
  }
  /// Returns vec![(definite, key)]
  fn get_own_keys(&'a self, _analyzer: &Analyzer<'a>) -> Option<Vec<(bool, Entity<'a>)>> {
    None
  }
  fn get_constructor_prototype(
    &'a self,
    _analyzer: &Analyzer<'a>,
    _dep: Dep<'a>,
  ) -> Option<(Dep<'a>, ObjectPrototype<'a>, ObjectPrototype<'a>)> {
    None
  }

  fn test_typeof(&self) -> TypeofResult;
  fn test_truthy(&self) -> Option<bool>;
  fn test_nullish(&self) -> Option<bool>;
  fn test_is_undefined(&self) -> Option<bool> {
    let t = self.test_typeof();
    match (t == TypeofResult::Undefined, t.contains(TypeofResult::Undefined)) {
      (true, _) => Some(true),
      (false, true) => None,
      (false, false) => Some(false),
    }
  }

  fn destruct_as_array(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    length: usize,
    need_rest: bool,
  ) -> (Vec<Entity<'a>>, Option<Entity<'a>>, Dep<'a>) {
    let (mut elements, rest, dep) = self.iterate(analyzer, dep);
    let iterated_len = elements.len();
    let extras = match iterated_len.cmp(&length) {
      Ordering::Equal => Vec::new(),
      Ordering::Greater => elements.split_off(length),
      Ordering::Less => {
        elements.resize(length, rest.unwrap_or(analyzer.factory.undefined));
        Vec::new()
      }
    };
    for element in &mut elements {
      *element = analyzer.factory.computed(*element, dep);
    }

    let rest_arr = need_rest.then(|| {
      let rest_arr = analyzer.new_empty_array();
      rest_arr.elements.borrow_mut().extend(extras);
      if let Some(rest) = rest {
        rest_arr.init_rest(rest);
      }
      analyzer.factory.computed(rest_arr.into(), dep)
    });

    (elements, rest_arr, dep)
  }

  fn iterate_result_union(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
  ) -> Option<Entity<'a>> {
    let (elements, rest, deps) = self.iterate(analyzer, dep);
    if let Some(rest) = rest {
      let mut result = allocator::Vec::from_iter_in(elements.iter().copied(), analyzer.allocator);
      result.push(rest);
      Some(analyzer.factory.computed_union(result, deps))
    } else if !elements.is_empty() {
      Some(analyzer.factory.computed_union(
        allocator::Vec::from_iter_in(elements.iter().copied(), analyzer.allocator),
        deps,
      ))
    } else {
      None
    }
  }

  fn call_as_getter(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    this: Entity<'a>,
  ) -> Entity<'a> {
    self.call(analyzer, dep, this, analyzer.factory.empty_arguments)
  }

  fn call_as_setter(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    this: Entity<'a>,
    value: Entity<'a>,
  ) -> Entity<'a> {
    self.call(
      analyzer,
      dep,
      this,
      analyzer.factory.arguments(analyzer.factory.vec1((false, value))),
    )
  }

  /// If it is a shared value, reference equality doesn't mean anything.
  fn is_shared_value(&self) -> bool {
    false
  }
}

impl<'a, T: ValueTrait<'a> + 'a + ?Sized> CustomDepTrait<'a> for &'a T {
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    (*self).consume(analyzer)
  }
}

pub type Value<'a> = &'a (dyn ValueTrait<'a> + 'a);
