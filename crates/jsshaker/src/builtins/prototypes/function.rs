use super::{BuiltinPrototype, object::create_object_prototype};
use crate::{analyzer::Factory, init_prototype, value::arguments::ArgumentsValue};

pub fn create_function_prototype<'a>(factory: &Factory<'a>) -> BuiltinPrototype<'a> {
  init_prototype!("Function", create_object_prototype(factory), {
    "apply" => factory.implemented_builtin_fn("Function::apply", |analyzer, dep, this, args| {
      let args_arg = {
        let arg = args.get(analyzer, 1);
        match arg.test_is_undefined() {
          Some(true) => analyzer.factory.empty_arguments,
          Some(false) => ArgumentsValue::from_value(analyzer, arg, dep),
          None => analyzer.factory.unknown_arguments,
        }
      };
      let this_arg = args.get(analyzer, 0);
      this.call(analyzer, dep, this_arg, args_arg)
    }),
    "call" => factory.implemented_builtin_fn("Function::call", |analyzer, dep, this, args| {
      let (this_arg, args_arg) = args.split_at(analyzer, 1);
      this.call(analyzer, dep, this_arg[0], args_arg)
    }),
    "bind" => factory.implemented_builtin_fn("Function::bind", |analyzer, dep, func, args| {
      let (bound_this, bound_args) = args.split_at(analyzer, 1);
      let bound_this = bound_this[0];
      let bound_fn = analyzer.factory.implemented_consumable_fn("Function::bound_fn", move |analyzer, dep, this, args| {
        let this = analyzer.op_undefined_or(bound_this, this);
        let args = ArgumentsValue::from_concatenate(analyzer, bound_args, args);
        func.call(analyzer, dep, this, args)
      });
      analyzer.factory.computed(bound_fn, dep)
    }),
    "length" => factory.unknown_number,
    "arguments" => factory.unknown,
    "caller" => factory.unknown,
    "name" => factory.unknown_string,
    "prototype" => factory.unknown,
  })
}
