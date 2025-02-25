use crate::{
  builtins::{constants::OBJECT_CONSTRUCTOR_OBJECT_ID, Builtins},
  entity::{Entity, ObjectPropertyValue, TypeofResult},
  init_namespace,
};
use std::borrow::BorrowMut;

impl<'a> Builtins<'a> {
  pub fn init_object_constructor(&mut self) {
    let factory = self.factory;

    let object =
      factory.builtin_object(OBJECT_CONSTRUCTOR_OBJECT_ID, &self.prototypes.function, false);
    object.init_rest(ObjectPropertyValue::Field(factory.immutable_unknown, true));

    init_namespace!(object, {
      "prototype" => factory.immutable_unknown,
      "assign" => self.create_object_assign_impl(),
      "keys" => self.create_object_keys_impl(),
      "values" => self.create_object_values_impl(),
      "entries" => self.create_object_entries_impl(),
      "freeze" => self.create_object_freeze_impl(),
    });

    self.globals.borrow_mut().insert("Object", object);
  }

  fn create_object_assign_impl(&self) -> Entity<'a> {
    self.factory.implemented_builtin_fn("Object.assign", |analyzer, dep, _, args| {
      let (known, rest, deps) = args.iterate(analyzer, dep);

      if known.len() < 2 {
        return analyzer.factory.computed_unknown((dep, args));
      }

      let target = known[0];

      let mut assign = |source: Entity<'a>, indeterminate: bool| {
        let (properties, deps) = source.enumerate_properties(analyzer, dep);
        for (definite, key, value) in properties {
          if indeterminate || !definite {
            analyzer.push_indeterminate_cf_scope();
          }
          target.set_property(analyzer, deps, key, value);
          if indeterminate || !definite {
            analyzer.pop_cf_scope();
          }
        }
      };

      for source in &known[1..] {
        assign(*source, false);
      }
      if let Some(rest) = rest {
        assign(rest, true);
      }

      analyzer.factory.computed(target, deps)
    })
  }

  fn create_object_keys_impl(&self) -> Entity<'a> {
    self.factory.implemented_builtin_fn("Object.keys", |analyzer, dep, _, args| {
      let object = args.destruct_as_array(analyzer, dep, 1, false).0[0];
      let (properties, deps) = object.enumerate_properties(analyzer, dep);

      let array = analyzer.new_empty_array();

      for (_, key, value) in properties {
        if key.test_typeof().contains(TypeofResult::String) {
          array.init_rest(analyzer.factory.computed(key.get_to_string(analyzer), value));
        }
      }

      analyzer.factory.computed(array, deps)
    })
  }

  fn create_object_values_impl(&self) -> Entity<'a> {
    self.factory.implemented_builtin_fn("Object.values", |analyzer, dep, _, args| {
      let object = args.destruct_as_array(analyzer, dep, 1, false).0[0];
      let (properties, deps) = object.enumerate_properties(analyzer, dep);

      let array = analyzer.new_empty_array();

      for (_, _, value) in properties {
        array.init_rest(value);
      }

      analyzer.factory.computed(array, deps)
    })
  }

  fn create_object_entries_impl(&self) -> Entity<'a> {
    self.factory.implemented_builtin_fn("Object.entries", |analyzer, dep, _, args| {
      let object = args.destruct_as_array(analyzer, dep, 1, false).0[0];
      let (properties, deps) = object.enumerate_properties(analyzer, dep);

      let array = analyzer.new_empty_array();

      for (_, key, value) in properties {
        let entry = analyzer.new_empty_array();
        entry.push_element(key.get_to_string(analyzer));
        entry.push_element(value);
        array.init_rest(entry);
      }

      analyzer.factory.computed(array, deps)
    })
  }

  fn create_object_freeze_impl(&self) -> Entity<'a> {
    self.factory.implemented_builtin_fn("Object.freeze", |analyzer, dep, _, args| {
      let object = args.destruct_as_array(analyzer, dep, 1, false).0[0];
      // TODO: Actually freeze the object
      analyzer.factory.computed(object, dep)
    })
  }
}
