use super::{BuiltinPrototype, object::create_object_prototype};
use crate::{analyzer::Factory, init_prototype};

pub fn create_number_prototype<'a>(factory: &Factory<'a>) -> BuiltinPrototype<'a> {
  init_prototype!("Number", create_object_prototype(factory), {
    "toExponential" => factory.pure_fn_returns_string,
    "toFixed" => factory.pure_fn_returns_string,
    "toLocaleString" => factory.pure_fn_returns_string,
    "toPrecision" => factory.pure_fn_returns_string,
    "valueOf" => factory.pure_fn_returns_number,
  })
}
