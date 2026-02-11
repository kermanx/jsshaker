use crate::{
  Analyzer, builtin_symbol,
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
      "asyncIterator" => builtin_symbol!(0),
      "hasInstance" => builtin_symbol!(1),
      "isConcatSpreadable" => builtin_symbol!(2),
      "iterator" => builtin_symbol!(3),
      "match" => builtin_symbol!(4),
      "matchAll" => builtin_symbol!(5),
      "replace" => builtin_symbol!(6),
      "search" => builtin_symbol!(7),
      "species" => builtin_symbol!(8),
      "split" => builtin_symbol!(9),
      "toPrimitive" => builtin_symbol!(10),
      "toStringTag" => builtin_symbol!(11),
      "unscopables" => builtin_symbol!(12),
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
