use super::{BuiltinPrototype, object::create_object_prototype};
use crate::{analyzer::Factory, init_prototype};

pub fn create_promise_prototype<'a>(factory: &Factory<'a>) -> BuiltinPrototype<'a> {
  init_prototype!("Promise", create_object_prototype(factory), {
    "finally" => factory.immutable_unknown,
    "then" => factory.immutable_unknown,
    "catch" => factory.immutable_unknown,
  })
}
