use crate::{
  builtins::Builtins,
  init_namespace,
  value::{ObjectPropertyValue, ObjectPrototype},
};

impl Builtins<'_> {
  pub fn init_math_object(&mut self) {
    let factory = self.factory;

    let object = factory.builtin_object(ObjectPrototype::Builtin(&self.prototypes.object), false);
    object.init_rest(factory, ObjectPropertyValue::Field(factory.unknown, true));

    init_namespace!(object, factory, {
      // Value properties (constants)
      "E" => factory.unknown_number,
      "LN10" => factory.unknown_number,
      "LN2" => factory.unknown_number,
      "LOG10E" => factory.unknown_number,
      "LOG2E" => factory.unknown_number,
      "PI" => factory.unknown_number,
      "SQRT1_2" => factory.unknown_number,
      "SQRT2" => factory.unknown_number,

      // Static methods
      "abs" => factory.pure_fn_returns_number,
      "acos" => factory.pure_fn_returns_number,
      "acosh" => factory.pure_fn_returns_number,
      "asin" => factory.pure_fn_returns_number,
      "asinh" => factory.pure_fn_returns_number,
      "atan" => factory.pure_fn_returns_number,
      "atanh" => factory.pure_fn_returns_number,
      "atan2" => factory.pure_fn_returns_number,
      "cbrt" => factory.pure_fn_returns_number,
      "ceil" => factory.pure_fn_returns_number,
      "clz32" => factory.pure_fn_returns_number,
      "cos" => factory.pure_fn_returns_number,
      "cosh" => factory.pure_fn_returns_number,
      "exp" => factory.pure_fn_returns_number,
      "expm1" => factory.pure_fn_returns_number,
      "floor" => factory.pure_fn_returns_number,
      "fround" => factory.pure_fn_returns_number,
      "hypot" => factory.pure_fn_returns_number,
      "imul" => factory.pure_fn_returns_number,
      "log" => factory.pure_fn_returns_number,
      "log1p" => factory.pure_fn_returns_number,
      "log10" => factory.pure_fn_returns_number,
      "log2" => factory.pure_fn_returns_number,
      "max" => factory.pure_fn_returns_number,
      "min" => factory.pure_fn_returns_number,
      "pow" => factory.pure_fn_returns_number,
      "random" => factory.pure_fn_returns_number,
      "round" => factory.pure_fn_returns_number,
      "sign" => factory.pure_fn_returns_number,
      "sin" => factory.pure_fn_returns_number,
      "sinh" => factory.pure_fn_returns_number,
      "sqrt" => factory.pure_fn_returns_number,
      "tan" => factory.pure_fn_returns_number,
      "tanh" => factory.pure_fn_returns_number,
      "trunc" => factory.pure_fn_returns_number,
    });

    self.globals.insert("Math", object.into());
  }
}
