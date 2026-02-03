mod array;
mod bigint;
mod boolean;
mod function;
mod null;
mod number;
mod object;
mod promise;
mod regexp;
mod string;
mod symbol;
mod utils;

use std::fmt;

use oxc::{allocator, span::Atom};

use super::Builtins;
use crate::{
  analyzer::{Analyzer, Factory},
  dep::Dep,
  entity::Entity,
  value::{LiteralValue, PropertyKeyValue, Value},
};

pub struct BuiltinPrototype<'a> {
  name: &'static str,
  fields: allocator::HashMap<'a, PropertyKeyValue<'a>, (bool, Entity<'a>)>,
}

impl fmt::Debug for BuiltinPrototype<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(format!("Prototype({})", self.name).as_str())
  }
}

impl<'a> BuiltinPrototype<'a> {
  pub fn new_in(factory: &Factory<'a>) -> Self {
    Self { name: "", fields: allocator::HashMap::new_in(factory.allocator) }
  }

  pub fn with_name(mut self, name: &'static str) -> Self {
    self.name = name;
    self
  }

  pub fn insert_string_keyed(
    &mut self,
    key: &'a Atom<'a>,
    is_getter: bool,
    value: impl Into<Entity<'a>>,
  ) {
    self.fields.insert(PropertyKeyValue::String(key), (is_getter, value.into()));
  }

  pub fn get_keyed(
    &self,
    analyzer: &Analyzer<'a>,
    key: PropertyKeyValue,
    this: Value<'a>,
  ) -> Option<Entity<'a>> {
    let (is_getter, value) = self.fields.get(&key).copied()?;
    Some(if is_getter { analyzer.factory.computed(value, this) } else { value })
  }

  pub fn get_literal_keyed(&self, key: LiteralValue) -> Option<Entity<'a>> {
    let (key, _) = key.into();
    self.fields.get(&key).map(|(_, v)| *v)
  }

  pub fn get_property(
    &self,
    analyzer: &Analyzer<'a>,
    target: Entity<'a>,
    key: Entity<'a>,
    dep: Dep<'a>,
  ) -> Entity<'a> {
    let dep = (dep, target, key);
    if let Some(key_literals) = key.get_literals(analyzer) {
      let mut values = analyzer.factory.vec();
      for &key_literal in &key_literals {
        if let Some(property) = self.get_literal_keyed(key_literal) {
          values.push(property);
        } else {
          values.push(analyzer.factory.unmatched_prototype_property);
        }
      }
      analyzer.factory.computed_union(values, dep)
    } else {
      analyzer.factory.computed_unknown(dep)
    }
  }
}

pub struct BuiltinPrototypes<'a> {
  pub array: BuiltinPrototype<'a>,
  pub bigint: BuiltinPrototype<'a>,
  pub boolean: BuiltinPrototype<'a>,
  pub function: BuiltinPrototype<'a>,
  pub null: BuiltinPrototype<'a>,
  pub number: BuiltinPrototype<'a>,
  pub object: BuiltinPrototype<'a>,
  pub promise: BuiltinPrototype<'a>,
  pub regexp: BuiltinPrototype<'a>,
  pub string: BuiltinPrototype<'a>,
  pub symbol: BuiltinPrototype<'a>,
}

impl<'a> Builtins<'a> {
  pub fn create_builtin_prototypes(factory: &Factory<'a>) -> &'a BuiltinPrototypes<'a> {
    factory.alloc(BuiltinPrototypes {
      array: array::create_array_prototype(factory),
      bigint: bigint::create_bigint_prototype(factory),
      boolean: boolean::create_boolean_prototype(factory),
      function: function::create_function_prototype(factory),
      null: null::create_null_prototype(factory),
      number: number::create_number_prototype(factory),
      object: object::create_object_prototype(factory),
      promise: promise::create_promise_prototype(factory),
      regexp: regexp::create_regexp_prototype(factory),
      string: string::create_string_prototype(factory),
      symbol: symbol::create_symbol_prototype(factory),
    })
  }
}
