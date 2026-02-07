pub mod array;
pub mod cacheable;
pub mod escaped;
mod function;
pub mod literal;
pub mod logical_result;
pub mod module_object;
pub mod never;
mod object;
pub mod primitive;
pub mod react_element;
mod typeof_result;
pub mod union;
pub mod unknown;
pub mod utils;

use cacheable::Cacheable;
pub use function::*;
pub use literal::LiteralValue;
pub use object::*;
use oxc::{semantic::SymbolId, span::Atom};
use rustc_hash::FxHashMap;
use std::{cmp::Ordering, fmt::Debug};
pub use typeof_result::TypeofResult;

use crate::{
  analyzer::Analyzer,
  dep::{CustomDepTrait, Dep},
  entity::Entity,
  value::literal::PossibleLiterals,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PropertyKeyValue<'a> {
  String(&'a Atom<'a>),
  Symbol(SymbolId),
}

impl<'a> PropertyKeyValue<'a> {
  pub fn is_special_key(&self) -> bool {
    matches!(self, PropertyKeyValue::String(s) if s.as_str() == "__proto__")
  }
}

#[derive(Debug)]
pub struct EnumeratedProperties<'a> {
  pub known: FxHashMap<PropertyKeyValue<'a>, (bool, Entity<'a>, Entity<'a>)>,
  pub unknown: Option<Entity<'a>>,
  pub dep: Dep<'a>,
}

impl<'a> EnumeratedProperties<'a> {
  pub fn into_dep(self, analyzer: &mut Analyzer<'a>) -> Dep<'a> {
    let mut entities = analyzer.factory.vec();
    for (_, key_entity, value_entity) in self.known.values() {
      entities.push(*key_entity);
      entities.push(*value_entity);
    }
    analyzer.factory.dep((entities, self.unknown, self.dep))
  }
}

/// (vec![known_elements], rest, dep)
pub type IteratedElements<'a> = (Vec<Entity<'a>>, Option<Entity<'a>>, Dep<'a>);

pub enum UnionHint {
  Unknown,
  Never,
  Object,
  Other,
}

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
    args: ArgumentsValue<'a>,
  ) -> Entity<'a>;
  fn construct(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    args: ArgumentsValue<'a>,
  ) -> Entity<'a>;
  fn jsx(&'a self, analyzer: &mut Analyzer<'a>, props: Entity<'a>) -> Entity<'a>;
  fn r#await(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) -> Entity<'a>;
  fn iterate(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) -> IteratedElements<'a>;

  fn coerce_string(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a>;
  fn coerce_number(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a>;
  fn coerce_boolean(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a>;
  fn coerce_property_key(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a>;
  fn coerce_jsx_child(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a>;

  fn get_shallow_dep(&'a self, _analyzer: &Analyzer<'a>) -> Option<Dep<'a>> {
    None
  }
  fn get_literals(&'a self, _analyzer: &Analyzer<'a>) -> Option<PossibleLiterals<'a>> {
    None
  }
  fn get_literal(&'a self, _analyzer: &Analyzer<'a>) -> Option<LiteralValue<'a>> {
    None
  }
  /// Returns vec![(definite, key)]
  fn get_own_keys(&'a self, _analyzer: &Analyzer<'a>) -> Option<Vec<(bool, Entity<'a>)>> {
    None
  }
  fn get_constructor_prototype(
    &'a self,
    _analyzer: &mut Analyzer<'a>,
    _dep: Dep<'a>,
  ) -> Option<(Dep<'a>, ObjectPrototype<'a>, ObjectPrototype<'a>)> {
    None
  }
  fn as_object(&'a self) -> Option<&'a ObjectValue<'a>> {
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
      analyzer.factory.arguments(analyzer.factory.alloc([value]), None),
    )
  }

  /// If it is a shared value, reference equality doesn't mean anything.
  fn is_shared_value(&self) -> bool {
    false
  }
  fn get_union_hint(&self) -> UnionHint {
    UnionHint::Other
  }

  fn as_cacheable(&self, analyzer: &Analyzer<'a>) -> Option<Cacheable<'a>>;
}

impl<'a, T: ValueTrait<'a> + 'a + ?Sized> CustomDepTrait<'a> for &'a T {
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    (*self).consume(analyzer)
  }
}

pub type Value<'a> = &'a (dyn ValueTrait<'a> + 'a);
