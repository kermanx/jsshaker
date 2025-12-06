use crate::value::{ObjectId, primitive::PrimitiveValue};

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
  // Object, Array, Function
  Object(ObjectId),
  BuiltinFn(&'static str),
}
