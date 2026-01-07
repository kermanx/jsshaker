use super::{BuiltinPrototype, object::create_object_prototype};
use crate::{
  analyzer::Factory,
  init_prototype,
  utils::CalleeNode,
  value::{ArgumentsValue, bound::BoundFunction},
};

pub fn create_function_prototype<'a>(factory: &Factory<'a>) -> BuiltinPrototype<'a> {
  init_prototype!("Function", create_object_prototype(factory), {
    "apply" => factory.implemented_builtin_fn("Function::apply", |analyzer, dep, this, args| {
      let this_arg = args.get(analyzer, 0);
      let arg = args.get(analyzer, 1);
      let args_arg = match arg.test_is_undefined() {
        Some(true) => analyzer.factory.empty_arguments,
        Some(false) => ArgumentsValue::from_value(analyzer, arg, dep),
        None => analyzer.factory.unknown_arguments,
      };
      let deps = analyzer.factory.dep((dep,arg.get_shallow_dep(analyzer)));
      this.call(analyzer, deps, this_arg, args_arg)
    }),
    "call" => factory.implemented_builtin_fn("Function::call", |analyzer, dep, this, args| {
      let (this_arg, args_arg) = args.split_at(analyzer, 1);
      this.call(analyzer, dep, this_arg[0], args_arg)
    }),
    "bind" => factory.implemented_builtin_fn("Function::bind", |analyzer, dep, target, args| {
      let (bound_this, bound_args) = args.split_at(analyzer, 1);
      let bound_this = bound_this[0];
      let span = analyzer.current_span();
      let bound_fn = analyzer.factory.alloc(BoundFunction {
        span,
        target,
        bound_this,
        bound_args,
      });
      analyzer.factory.computed(analyzer.new_function(CalleeNode::BoundFunction(bound_fn)).into(), (dep, args.get_shallow_dep(analyzer)))
    }),
    "length" => factory.unknown_number,
    "arguments" => factory.unknown,
    "caller" => factory.unknown,
    "name" => factory.unknown_string,
    "prototype" => factory.unknown,
  })
}
