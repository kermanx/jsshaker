use crate::{
  builtins::Builtins,
  entity::Entity,
  init_namespace,
  value::{ObjectPropertyValue, ObjectPrototype},
};

impl<'a> Builtins<'a> {
  pub fn init_array_constructor(&mut self) {
    let factory = self.factory;

    let object = factory.builtin_object(ObjectPrototype::Builtin(&self.prototypes.function), false);
    object.init_rest(factory, ObjectPropertyValue::Field(factory.unknown, true));

    init_namespace!(object, factory, {
      "prototype" => factory.unknown,
      "from" => self.create_array_from_impl(),
      "fromAsync" => factory.pure_fn_returns_unknown,
      "isArray" => factory.pure_fn_returns_boolean,
      "of" => self.create_array_of_impl(),
    });

    self.globals.insert("Array", object.into());
  }

  fn create_array_from_impl(&self) -> Entity<'a> {
    self.factory.implemented_builtin_fn("Array.from", |analyzer, dep, this, args| {
      let iterable = args.get(analyzer, 0);
      let map_fn = args.get(analyzer, 1);
      let this_arg = args.get(analyzer, 2);

      let array = analyzer.new_empty_array();

      let (elements, rest, dep) = iterable.iterate(analyzer, dep);
      let no_map_fn = map_fn.test_is_undefined();

      for (i, element) in elements.into_iter().enumerate() {
        let element = if no_map_fn != Some(true) {
          let index = analyzer.factory.number(i as f64, None);
          let args = analyzer.factory.arguments(analyzer.allocator.alloc([element, index]), None);
          let mapped = map_fn.call(analyzer, dep, this_arg, args);
          if no_map_fn == Some(false) { mapped } else { analyzer.factory.union((element, mapped)) }
        } else {
          element
        };
        array.push_element(element);
      }

      if let Some(rest) = rest {
        analyzer.push_non_det_cf_scope();
        let rest = if no_map_fn != Some(true) {
          let args = analyzer
            .factory
            .arguments(analyzer.allocator.alloc([rest, analyzer.factory.unknown_number]), None);
          let mapped = map_fn.call(analyzer, dep, this_arg, args);
          if no_map_fn == Some(false) { mapped } else { analyzer.factory.union((rest, mapped)) }
        } else {
          rest
        };
        array.init_rest(rest);
        analyzer.pop_cf_scope();
      }

      analyzer.factory.computed(array.into(), (this, dep))
    })
  }

  fn create_array_of_impl(&self) -> Entity<'a> {
    self.factory.implemented_builtin_fn("Array.of", |analyzer, dep, _, args| {
      let array = analyzer.new_empty_array();

      for element in args.elements.iter() {
        array.push_element(*element);
      }

      if let Some(rest) = args.rest {
        array.init_rest(rest);
      }

      analyzer.factory.computed(array.into(), dep)
    })
  }
}
