use crate::{Analyzer, entity::Entity, value::primitive::PrimitiveValue};

use super::LiteralValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cacheable<'a> {
  Unknown,
  Never,

  UnknownTruthy,
  UnknownFalsy,
  UnknownNullish,
  UnknownNonNullish,

  Literal(LiteralValue<'a>),
  Primitive(PrimitiveValue),
  // BuiltinFn(&'static str),
  // // Object, Array, Function
  // Object(ObjectId),
}

impl<'a> Cacheable<'a> {
  pub fn into_entity(self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    match self {
      Cacheable::Unknown => analyzer.factory.unknown,
      Cacheable::Never => analyzer.factory.never,
      Cacheable::UnknownTruthy => analyzer.factory.unknown_truthy,
      Cacheable::UnknownFalsy => analyzer.factory.unknown_falsy,
      Cacheable::UnknownNullish => analyzer.factory.unknown_nullish,
      Cacheable::UnknownNonNullish => analyzer.factory.unknown_non_nullish,
      Cacheable::Literal(v) => analyzer.factory.alloc(v).into(),
      Cacheable::Primitive(v) => analyzer.factory.alloc(v).into(),
    }
  }
}
