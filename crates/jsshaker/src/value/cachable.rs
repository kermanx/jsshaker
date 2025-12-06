use crate::{Analyzer, entity::Entity, value::primitive::PrimitiveValue};

use super::LiteralValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cachable<'a> {
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

impl<'a> Cachable<'a> {
  pub fn into_entity(self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    match self {
      Cachable::Unknown => analyzer.factory.unknown,
      Cachable::Never => analyzer.factory.never,
      Cachable::UnknownTruthy => analyzer.factory.unknown_truthy,
      Cachable::UnknownFalsy => analyzer.factory.unknown_falsy,
      Cachable::UnknownNullish => analyzer.factory.unknown_nullish,
      Cachable::UnknownNonNullish => analyzer.factory.unknown_non_nullish,
      Cachable::Literal(v) => analyzer.factory.alloc(v).into(),
      Cachable::Primitive(v) => analyzer.factory.alloc(v).into(),
    }
  }
}
