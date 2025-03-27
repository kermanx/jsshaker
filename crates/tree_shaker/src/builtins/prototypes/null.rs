use super::BuiltinPrototype;
use crate::analyzer::Factory;

pub fn create_null_prototype<'a>(factory: &Factory<'a>) -> BuiltinPrototype<'a> {
  BuiltinPrototype::new_in(factory).with_name("null")
}
