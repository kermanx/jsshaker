use super::{BuiltinPrototype, object::create_object_prototype};
use crate::{
  analyzer::Factory,
  init_prototype,
  value::{ObjectId, arguments::ArgumentsValue},
};

pub fn create_function_prototype<'a>(factory: &Factory<'a>) -> BuiltinPrototype<'a> {
  init_prototype!("Function", create_object_prototype(factory), {
    "apply" => factory.implemented_builtin_fn("Function::apply", |analyzer, dep, this, args| {
      let mut args = args.destruct_as_array(analyzer, dep, 2, false).0;
      let args_arg = {
        let arg = args.pop().unwrap();
        let cf_scope = analyzer.scoping.cf.current_id();
        // This can be any value
        let arguments_object_id = ObjectId::from_usize(0);
        match arg.test_is_undefined() {
          Some(true) => analyzer.factory.array(cf_scope, arguments_object_id).into(),
          Some(false) => arg,
          None => analyzer.factory.union((
            arg,
            analyzer.factory.array(cf_scope, arguments_object_id).into(),
          )),
        }
      };
      let this_arg = args.pop().unwrap();
      this.call(analyzer, dep, this_arg, args_arg)
    }),
    "call" => factory.implemented_builtin_fn("Function::call", |analyzer, dep, this, args| {
      let (this_arg, args_arg, _deps) = args.destruct_as_array(analyzer, dep, 1, true);
      this.call(analyzer, dep, this_arg[0], args_arg.unwrap())
    }),
    "bind" => factory.implemented_builtin_fn("Function::bind", |analyzer, dep, func, args| {
      let (bound_this, bound_args, _deps) = args.destruct_as_array(analyzer, dep, 1, true);
      let bound_this = bound_this[0];
      let bound_fn = analyzer.factory.implemented_consumable_fn("Function::bound_fn", move |analyzer, dep, this, args| {
        let this = analyzer.op_undefined_or(bound_this, this);
        let (args, dep) = ArgumentsValue::from_concatenate(analyzer, bound_args.unwrap(), args, dep);
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
