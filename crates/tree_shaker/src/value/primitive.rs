use super::{
  EnumeratedProperties, IteratedElements, TypeofResult, ValueTrait, consumed_object,
  never::NeverValue,
};
use crate::{analyzer::Analyzer, builtins::BuiltinPrototype, dep::Dep, entity::Entity};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveValue {
  // TODO: NumericString, NoneEmptyString, ...
  Mixed,
  String,
  Number,
  BigInt,
  Boolean,
  Symbol,
}

impl<'a> ValueTrait<'a> for PrimitiveValue {
  fn consume(&'a self, _analyzer: &mut Analyzer<'a>) {}

  fn unknown_mutate(&'a self, _analyzer: &mut Analyzer<'a>, _dep: Dep<'a>) {
    // No effect
  }

  fn get_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    key: Entity<'a>,
  ) -> Entity<'a> {
    // TODO: PrimitiveValue::String
    if *self == PrimitiveValue::Mixed || *self == PrimitiveValue::String {
      analyzer.factory.computed_unknown((self, dep, key))
    } else {
      let prototype = self.get_prototype(analyzer);
      prototype.get_property(analyzer, self.into(), key, dep)
    }
  }

  fn set_property(
    &'a self,
    _analyzer: &mut Analyzer<'a>,
    _dep: Dep<'a>,
    _key: Entity<'a>,
    _value: Entity<'a>,
  ) {
    // No effect
  }

  fn enumerate_properties(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
  ) -> EnumeratedProperties<'a> {
    if *self == PrimitiveValue::String {
      (
        vec![(false, analyzer.factory.unknown_string, analyzer.factory.unknown_string)],
        analyzer.dep((self, dep)),
      )
    } else {
      (vec![], analyzer.dep((self, dep)))
    }
  }

  fn delete_property(&'a self, _analyzer: &mut Analyzer<'a>, _dep: Dep<'a>, _key: Entity<'a>) {
    // No effect
  }

  fn call(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    this: Entity<'a>,
    args: Entity<'a>,
  ) -> Entity<'a> {
    analyzer.throw_builtin_error("Cannot call non-object");
    if analyzer.config.preserve_exceptions {
      consumed_object::call(self, analyzer, dep, this, args)
    } else {
      analyzer.factory.never
    }
  }

  fn construct(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    args: Entity<'a>,
  ) -> Entity<'a> {
    analyzer.throw_builtin_error("Cannot construct non-object");
    if analyzer.config.preserve_exceptions {
      consumed_object::construct(self, analyzer, dep, args)
    } else {
      analyzer.factory.never
    }
  }

  fn jsx(&'a self, analyzer: &mut Analyzer<'a>, props: Entity<'a>) -> Entity<'a> {
    analyzer.factory.computed_unknown((self, props))
  }

  fn r#await(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) -> Entity<'a> {
    analyzer.factory.entity_with_dep(self, dep)
  }

  fn iterate(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) -> IteratedElements<'a> {
    if *self == PrimitiveValue::String {
      return (vec![], Some(analyzer.factory.unknown), analyzer.dep((self, dep)));
    }
    analyzer.throw_builtin_error("Cannot iterate non-object");
    if analyzer.config.preserve_exceptions {
      self.consume(analyzer);
      consumed_object::iterate(analyzer, dep)
    } else {
      NeverValue.iterate(analyzer, dep)
    }
  }

  fn get_typeof(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    if let Some(str) = self.test_typeof().to_string() {
      analyzer.factory.string(str)
    } else {
      analyzer.factory.unknown_string
    }
  }

  fn get_to_string(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.unknown_string
  }

  fn get_to_numeric(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.unknown
  }

  fn get_to_boolean(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    match self.test_truthy() {
      Some(val) => analyzer.factory.boolean(val),
      None => analyzer.factory.unknown_boolean,
    }
  }

  fn get_to_property_key(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.unknown
  }

  fn get_to_jsx_child(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    if matches!(self, PrimitiveValue::Mixed | PrimitiveValue::String | PrimitiveValue::Number) {
      analyzer.factory.unknown_string
    } else {
      analyzer.factory.string("")
    }
  }
  fn get_own_keys(&'a self, _analyzer: &Analyzer<'a>) -> Option<Vec<(bool, Entity<'a>)>> {
    match self {
      PrimitiveValue::String => None,
      _ => Some(vec![]),
    }
  }

  fn test_typeof(&self) -> TypeofResult {
    match self {
      PrimitiveValue::String => TypeofResult::String,
      PrimitiveValue::Number => TypeofResult::Number,
      PrimitiveValue::BigInt => TypeofResult::BigInt,
      PrimitiveValue::Boolean => TypeofResult::Boolean,
      PrimitiveValue::Symbol => TypeofResult::Symbol,
      PrimitiveValue::Mixed => TypeofResult::_Unknown,
    }
  }

  fn test_truthy(&self) -> Option<bool> {
    match self {
      PrimitiveValue::Symbol => Some(true),
      _ => None,
    }
  }

  fn test_nullish(&self) -> Option<bool> {
    Some(false)
  }
}

impl<'a> PrimitiveValue {
  fn get_prototype(&self, analyzer: &mut Analyzer<'a>) -> &'a BuiltinPrototype<'a> {
    match self {
      PrimitiveValue::String => &analyzer.builtins.prototypes.string,
      PrimitiveValue::Number => &analyzer.builtins.prototypes.number,
      PrimitiveValue::BigInt => &analyzer.builtins.prototypes.bigint,
      PrimitiveValue::Boolean => &analyzer.builtins.prototypes.boolean,
      PrimitiveValue::Symbol => &analyzer.builtins.prototypes.symbol,
      PrimitiveValue::Mixed => unreachable!("Cannot get prototype of mixed primitive"),
    }
  }
}
