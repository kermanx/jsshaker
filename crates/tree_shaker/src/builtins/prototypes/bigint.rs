use super::{BuiltinPrototype, object::create_object_prototype};
use crate::analyzer::Factory;

pub fn create_bigint_prototype<'a>(factory: &Factory<'a>) -> BuiltinPrototype<'a> {
  create_object_prototype(factory).with_name("BigInt")
}
