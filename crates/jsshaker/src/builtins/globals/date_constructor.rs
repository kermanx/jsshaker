use crate::{
  builtins::Builtins,
  init_object,
  value::{ObjectPropertyValue, ObjectPrototype, escaped},
};

impl Builtins<'_> {
  pub fn init_date_constructor(&mut self) {
    let factory = self.factory;

    let statics =
      factory.builtin_object(ObjectPrototype::Builtin(&self.prototypes.function), false);
    statics.init_rest(factory, ObjectPropertyValue::Field(factory.unknown, true));

    init_object!(statics, factory, {
      "prototype" => factory.unknown,
      // Static methods
      "now" => factory.pure_fn_returns_number,
      "parse" => factory.pure_fn_returns_number,
      "UTC" => factory.pure_fn_returns_number,
    });

    self.globals.insert(
      "Date",
      self.factory.implemented_builtin_fn_with_statics("Date", escaped::builtin_call, statics),
    );
  }
}
