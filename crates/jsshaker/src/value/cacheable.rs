use oxc::allocator;

use crate::{
  entity::Entity,
  module::ModuleId,
  utils::{CalleeInstanceId, skip_hash_eq::SkipHashEq},
  value::{ObjectId, array::ArrayId, primitive::PrimitiveValue},
};

use super::LiteralValue;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Cacheable<'a> {
  Unknown,
  Never,

  String(&'a str),
  Literal(LiteralValue<'a>),
  Primitive(PrimitiveValue),
  Object(ObjectId),
  Array(ArrayId),
  ModuleObject(ModuleId),
  Function(CalleeInstanceId),
  BuiltinFn(&'static str),

  Union(allocator::Vec<'a, (Cacheable<'a>, SkipHashEq<Entity<'a>>)>),
}

impl<'a> Cacheable<'a> {
  pub fn is_copyable(&self) -> bool {
    match self {
      Self::Array(_) | Self::Object(_) => false,
      Self::Union(u) => u.iter().all(|(c, _)| c.is_copyable()),
      _ => true,
    }
  }

  pub fn is_compatible(&self, other: &Cacheable<'a>) -> bool {
    match (self, other) {
      (Cacheable::Unknown, _) | (_, Cacheable::Never) => true,

      (Cacheable::Primitive(PrimitiveValue::Mixed), Cacheable::Primitive(_)) => true,
      (Cacheable::Primitive(p), Cacheable::Literal(l)) => p.is_compatible(l),

      (c1, Cacheable::Union(u2)) => u2.iter().all(|(c2, _)| c1.is_compatible(c2)),
      (Cacheable::Union(u1), c2) => u1.iter().any(|(c1, _)| c1.is_compatible(c2)),

      (v1, v2) => v1 == v2,
    }
  }
}
