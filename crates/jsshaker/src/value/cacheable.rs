use oxc::allocator::{self, Allocator};

use crate::{
  module::ModuleId,
  utils::CalleeInstanceId,
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

  Union(allocator::Vec<'a, Cacheable<'a>>),
}

impl<'a> Cacheable<'a> {
  pub fn union(self, allocator: &'a Allocator, other: Cacheable<'a>) -> Cacheable<'a> {
    if self == other {
      return self;
    }
    match (self, other) {
      (Cacheable::Unknown, _) | (_, Cacheable::Unknown) => Cacheable::Unknown,
      (Cacheable::Never, v) | (v, Cacheable::Never) => v,
      (Cacheable::Union(mut u), Cacheable::Union(u2)) => {
        for v in u2 {
          if !u.contains(&v) {
            u.push(v);
          }
        }
        Cacheable::Union(u)
      }
      (Cacheable::Union(mut u), v) | (v, Cacheable::Union(mut u)) => {
        if !u.contains(&v) {
          u.push(v);
        }
        Cacheable::Union(u)
      }
      (v1, v2) => Cacheable::Union(allocator::Vec::from_array_in([v1, v2], allocator)),
    }
  }

  pub fn is_copyable(&self) -> bool {
    match self {
      // Enable identity-based caching for objects/arrays
      // Object and Array IDs are copyable and provide identity-based equality
      Self::Array(_) | Self::Object(_) => true,
      Self::Union(u) => u.iter().all(|c| c.is_copyable()),
      _ => true,
    }
  }

  pub fn is_compatible(&self, other: &Cacheable<'a>) -> bool {
    match (self, other) {
      (Cacheable::Unknown, _) | (_, Cacheable::Never) => true,

      (Cacheable::Primitive(PrimitiveValue::Mixed), Cacheable::Primitive(_)) => true,
      (Cacheable::Primitive(p), Cacheable::Literal(l)) => p.is_compatible(l),

      (c1, Cacheable::Union(u2)) => u2.iter().all(|c2| c1.is_compatible(c2)),
      (Cacheable::Union(u1), c2) => u1.iter().any(|c1| c1.is_compatible(c2)),

      (v1, v2) => v1 == v2,
    }
  }
}
