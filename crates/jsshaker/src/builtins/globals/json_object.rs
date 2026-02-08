use crate::{
  builtins::Builtins,
  entity::Entity,
  init_namespace,
  value::{ObjectPropertyValue, ObjectPrototype},
};

impl<'a> Builtins<'a> {
  pub fn init_json_object(&mut self) {
    let factory = self.factory;

    let object = factory.builtin_object(ObjectPrototype::Builtin(&self.prototypes.object), false);
    object.init_rest(factory, ObjectPropertyValue::Field(factory.unknown, true));

    init_namespace!(object, factory, {
      "parse" => self.create_json_parse_impl(),
      "stringify" => self.create_json_stringify_impl(),
    });

    self.globals.insert("JSON", object.into());
  }

  fn create_json_parse_impl(&self) -> Entity<'a> {
    self.factory.implemented_builtin_fn("JSON.parse", |analyzer, dep, _, args| {
      let text = args.get(analyzer, 0);
      let reviver = args.get(analyzer, 1);
      let deps = (dep, text, reviver);
      if reviver.test_is_undefined() != Some(true) {
        analyzer.include(deps);
        analyzer.factory.unknown
      } else {
        analyzer.factory.computed_unknown(deps)
      }
    })
  }

  fn create_json_stringify_impl(&self) -> Entity<'a> {
    let ret_type = self.factory.union((self.factory.unknown_string, self.factory.undefined));
    self.factory.implemented_builtin_fn("JSON.stringify", move |analyzer, dep, _, args| {
      let value = args.get(analyzer, 0);
      let replacer = args.get(analyzer, 1);
      let space = args.get(analyzer, 2);
      let deps = (dep, value, replacer, space);
      if analyzer.config.impure_json_stringify || replacer.test_is_undefined() != Some(true) {
        analyzer.include(deps);
        ret_type
      } else {
        analyzer.factory.computed(ret_type, deps)
      }
    })
  }
}
