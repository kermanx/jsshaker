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

  Literal(LiteralValue<'a>),
  Primitive(PrimitiveValue),
  Union(allocator::Vec<'a, Cacheable<'a>>),
  Object(ObjectId),
  Array(ArrayId),
  ModuleObject(ModuleId),
  Function(CalleeInstanceId),
  BuiltinFn(&'static str),
}

impl<'a> Cacheable<'a> {
  pub fn add(self, allocator: &'a Allocator, other: Cacheable<'a>) -> Cacheable<'a> {
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
}
