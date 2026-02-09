use crate::{
  Analyzer,
  builtins::Builtins,
  dep::Dep,
  entity::Entity,
  init_object,
  value::{ArgumentsValue, LiteralValue, ObjectPropertyValue, ObjectPrototype},
};

impl<'a> Builtins<'a> {
  pub fn init_symbol_constructor(&mut self) {
    let factory = self.factory;

    let statics = factory.builtin_object(ObjectPrototype::Builtin(&self.prototypes.function));
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
      "for" => self.create_symbol_for_impl(),
      "keyFor" => factory.pure_fn_returns_string,
    });

    self.globals.insert(
      "Symbol",
      self.factory.implemented_builtin_fn_with_statics("Symbol", symbol_constructor_impl, statics),
    );
  }

  fn create_symbol_for_impl(&self) -> Entity<'a> {
    self.factory.implemented_builtin_fn("Symbol.for", |analyzer, dep, _this, args| {
      // Symbol.for() returns the same symbol for the same key
      let key = args.get(analyzer, 0).coerce_string(analyzer);

      if let Some(LiteralValue::String(key_str, _)) = key.get_literal(analyzer) {
        // For constant keys, use the global symbol registry
        let symbol = analyzer
          .symbol_registry
          .get_or_create_global_symbol(key_str.as_str(), analyzer.allocator);
        analyzer.factory.computed(symbol.into(), (dep, key))
      } else {
        // For dynamic keys, return unknown symbol
        analyzer.factory.computed(analyzer.factory.unknown_symbol, (dep, key))
      }
    })
  }
}

fn symbol_constructor_impl<'a>(
  analyzer: &mut Analyzer<'a>,
  dep: Dep<'a>,
  _this: Entity<'a>,
  args: ArgumentsValue<'a>,
) -> Entity<'a> {
  let desc = args.get(analyzer, 0).coerce_string(analyzer);
  let symbol_id = analyzer.symbol_registry.alloc_symbol_id();
  analyzer.factory.computed(analyzer.factory.symbol(symbol_id), (dep, desc))
}
