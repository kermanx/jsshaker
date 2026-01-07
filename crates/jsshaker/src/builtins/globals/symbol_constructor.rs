use crate::{
  builtins::Builtins,
  init_object,
  value::{ObjectPropertyValue, ObjectPrototype},
};

impl Builtins<'_> {
  pub fn init_symbol_constructor(&mut self) {
    let factory = self.factory;

    let statics =
      factory.builtin_object(ObjectPrototype::Builtin(&self.prototypes.function), false);
    statics.init_rest(factory, ObjectPropertyValue::Field(factory.unknown, true));

    init_object!(statics, factory, {
      "prototype" => factory.unknown,
      // Well-known symbols
      "asyncIterator" => factory.unknown_symbol,
      "hasInstance" => factory.unknown_symbol,
      "isConcatSpreadable" => factory.unknown_symbol,
      "iterator" => factory.unknown_symbol,
      "match" => factory.unknown_symbol,
      "matchAll" => factory.unknown_symbol,
      "replace" => factory.unknown_symbol,
      "search" => factory.unknown_symbol,
      "species" => factory.unknown_symbol,
      "split" => factory.unknown_symbol,
      "toPrimitive" => factory.unknown_symbol,
      "toStringTag" => factory.unknown_symbol,
      "unscopables" => factory.unknown_symbol,
      // Static methods
      "for" => factory.pure_fn_returns_symbol,
      "keyFor" => factory.pure_fn_returns_string,
    });

    self.globals.insert(
      "Symbol",
      self.factory.implemented_builtin_fn_with_statics(
        "Symbol",
        |analyzer, dep, _, args| {
          let description = args.get(analyzer, 0);
          analyzer.factory.computed_unknown((dep, description))
        },
        statics,
      ),
    );
  }
}
