use std::borrow::BorrowMut;

use crate::{
  Analyzer,
  builtins::Builtins,
  entity::Entity,
  init_object,
  scope::CfScopeKind,
  value::{LiteralValue, ObjectPropertyValue, ObjectPrototype, TypeofResult, consumed_object},
};

impl<'a> Builtins<'a> {
  pub fn init_object_constructor(&mut self) {
    let factory = self.factory;

    let statics =
      factory.builtin_object(ObjectPrototype::Builtin(&self.prototypes.function), false);
    statics.init_rest(factory, ObjectPropertyValue::Field(factory.unknown, true));

    init_object!(statics, factory, {
      "prototype" => factory.unknown,
      "assign" => self.create_object_assign_impl(),
      "keys" => self.create_object_keys_impl(),
      "values" => self.create_object_values_impl(),
      "entries" => self.create_object_entries_impl(),
      "freeze" => self.create_object_freeze_impl(),
      "defineProperty" => self.create_object_define_property_impl(),
      "create" => self.create_object_create_impl(),
      "is" => self.create_object_is_impl(),
      "preventExtensions" => factory.unknown,
      "seal" => factory.unknown,
      "getOwnPropertyNames" => factory.unknown,
      "getOwnPropertySymbols" => factory.unknown,
      "getOwnPropertyDescriptor" => factory.unknown,
      "isExtensible" => factory.pure_fn_returns_boolean,
      "isFrozen" => factory.pure_fn_returns_boolean,
      "isSealed" => factory.pure_fn_returns_boolean,
    });

    self.globals.borrow_mut().insert(
      "Object",
      self.factory.implemented_builtin_fn_with_statics(
        "Object",
        consumed_object::builtin_call,
        statics,
      ),
    );
  }

  fn create_object_assign_impl(&self) -> Entity<'a> {
    self.factory.implemented_builtin_fn("Object.assign", |analyzer, dep, _, args| {
      let target = args.get(analyzer, 0);

      if target.test_typeof().intersects(TypeofResult::_Primitive) {
        analyzer.push_non_det_cf_scope();
        let mut deps = analyzer.factory.vec();
        for source in args.elements.iter().skip(1) {
          deps.push(source.enumerate_properties(analyzer, dep).into_dep(analyzer));
        }
        if let Some(rest) = args.rest {
          deps.push(rest.enumerate_properties(analyzer, dep).into_dep(analyzer));
        }
        target.unknown_mutate(analyzer, (dep, deps));
        analyzer.pop_cf_scope();
        return analyzer.factory.computed_unknown(target);
      }

      let mut deps = analyzer.factory.vec();
      let mut assign = |analyzer: &mut Analyzer<'a>, source: Entity<'a>, non_det: bool| {
        let enumerated = source.enumerate_properties(analyzer, dep);
        for (definite, key, value) in enumerated.known.into_values() {
          if !definite {
            analyzer.cf_scope_mut().exited = None;
          }
          target.set_property(analyzer, enumerated.dep, key, value);
          if !non_det {
            analyzer.cf_scope_mut().exited = Some(false);
          }
        }
        if let Some(unknown) = enumerated.unknown {
          target.set_property(analyzer, enumerated.dep, analyzer.factory.unknown_string, unknown);
        }
        deps.push(enumerated.dep);
      };

      analyzer.push_cf_scope(CfScopeKind::NonDet, false);
      for source in args.elements.iter().skip(1) {
        assign(analyzer, *source, false);
      }
      if let Some(rest) = args.rest {
        analyzer.cf_scope_mut().exited = None;
        assign(analyzer, rest, true);
      }
      analyzer.pop_cf_scope();

      analyzer.factory.computed(target, (dep, deps))
    })
  }

  fn create_object_keys_impl(&self) -> Entity<'a> {
    self.factory.implemented_builtin_fn("Object.keys", |analyzer, dep, _, args| {
      let object = args.get(analyzer, 0);
      let array = analyzer.new_empty_array();
      if let Some(keys) = object.get_own_keys(analyzer) {
        for (_, key) in keys {
          if key.test_typeof().contains(TypeofResult::String) {
            array.init_rest(key);
          }
        }
      } else {
        array.init_rest(analyzer.factory.unknown_string);
      }

      analyzer.factory.computed(array.into(), (dep, object.get_shallow_dep(analyzer)))
    })
  }

  fn create_object_values_impl(&self) -> Entity<'a> {
    self.factory.implemented_builtin_fn("Object.values", |analyzer, dep, _, args| {
      let object = args.get(analyzer, 0);
      let enumerated = object.enumerate_properties(analyzer, dep);

      let array = analyzer.new_empty_array();

      for (_, _, value) in enumerated.known.into_values() {
        array.init_rest(value);
      }

      if let Some(unknown) = enumerated.unknown {
        array.init_rest(unknown);
      }

      analyzer.factory.computed(array.into(), enumerated.dep)
    })
  }

  fn create_object_entries_impl(&self) -> Entity<'a> {
    self.factory.implemented_builtin_fn("Object.entries", |analyzer, dep, _, args| {
      let object = args.get(analyzer, 0);
      let enumerated = object.enumerate_properties(analyzer, dep);

      let array = analyzer.new_empty_array();

      for (_, key, value) in enumerated.known.into_values() {
        let entry = analyzer.new_empty_array();
        entry.push_element(key.coerce_string(analyzer));
        entry.push_element(value);
        array.init_rest(entry.into());
      }

      if let Some(unknown) = enumerated.unknown {
        let entry = analyzer.new_empty_array();
        entry.push_element(analyzer.factory.unknown_string);
        entry.push_element(unknown);
        array.init_rest(entry.into());
      }

      analyzer.factory.computed(array.into(), enumerated.dep)
    })
  }

  fn create_object_freeze_impl(&self) -> Entity<'a> {
    self.factory.implemented_builtin_fn("Object.freeze", |analyzer, dep, _, args| {
      let object = args.get(analyzer, 0);
      if analyzer.config.preserve_writablity {
        object.unknown_mutate(analyzer, dep);
        object
      } else {
        // TODO: Actually freeze the object
        analyzer.factory.computed(object, dep)
      }
    })
  }

  fn create_object_define_property_impl(&self) -> Entity<'a> {
    self.factory.implemented_builtin_fn("Object.defineProperty", |analyzer, dep, _, args| {
      let object = args.get(analyzer, 0);
      let key = args.get(analyzer, 1).coerce_property_key(analyzer);
      let descriptor = args.get(analyzer, 2);

      'trackable: {
        if analyzer.config.preserve_writablity {
          break 'trackable;
        }
        if key.get_literal(analyzer).is_none() {
          break 'trackable;
        }
        let enumerated = descriptor.enumerate_properties(analyzer, dep);
        let mut value = analyzer.factory.undefined;
        for (definite, key, value2) in enumerated.known.into_values() {
          if !definite {
            break 'trackable;
          }
          let Some(LiteralValue::String(key_str, _)) = key.get_literal(analyzer) else {
            break 'trackable;
          };
          match key_str.as_str() {
            "value" => {
              value = self.factory.computed(value2, (key, value));
            }
            "get" => {
              // FIXME: This is not safe, but OK for now.
              value = self.factory.computed_unknown((value2, key, value));
            }
            "set" | "enumerable" | "configurable" | "writable" => {
              // TODO: actually handle these
              value = self.factory.computed(value, (key, value2));
            }
            _ => {}
          }
        }
        object.set_property(
          analyzer,
          analyzer.factory.dep((enumerated.dep, descriptor.get_shallow_dep(analyzer))),
          key,
          value,
        );
        return object;
      }

      object.unknown_mutate(analyzer, (dep, key, descriptor));
      object
    })
  }

  fn create_object_create_impl(&self) -> Entity<'a> {
    self.factory.implemented_builtin_fn("Object.create", |analyzer, dep, _, args| {
      let proto = args.get(analyzer, 0);
      let properties = args.get(analyzer, 1);
      let deps = analyzer.dep((proto, dep));
      if properties.test_is_undefined() != Some(true) {
        // Has properties
        let enumerated = properties.enumerate_properties(analyzer, deps);
        if !enumerated.known.is_empty() || enumerated.unknown.is_some() {
          return analyzer.factory.computed_unknown((enumerated.dep, properties));
        }
      }
      let prototype = if proto.test_nullish() == Some(true) {
        ObjectPrototype::ImplicitOrNull
      } else {
        ObjectPrototype::Unknown(deps)
      };
      let mangling = analyzer.new_object_mangling_group();
      let object = analyzer.new_empty_object(prototype, Some(mangling));
      object.add_extra_dep(deps);
      object.into()
    })
  }

  fn create_object_is_impl(&self) -> Entity<'a> {
    self.factory.implemented_builtin_fn("Object.is", |analyzer, dep, _, args| {
      let lhs = args.get(analyzer, 0);
      let rhs = args.get(analyzer, 1);
      let (equality, mangle_constraint) = analyzer.op_strict_eq(lhs, rhs, true);

      if let Some(mangle_constraint) = mangle_constraint {
        self.factory.mangable(
          self.factory.computed(self.factory.boolean_maybe_unknown(equality), dep),
          (lhs, rhs),
          mangle_constraint,
        )
      } else {
        self.factory.computed(self.factory.boolean_maybe_unknown(equality), (dep, lhs, rhs))
      }
    })
  }
}
