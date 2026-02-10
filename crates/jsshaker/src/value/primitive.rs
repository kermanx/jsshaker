use super::{
  ArgumentsValue, EnumeratedProperties, IteratedElements, TypeofResult, ValueTrait,
  cacheable::Cacheable, escaped, never::NeverValue,
};
use crate::{
  analyzer::Analyzer, builtin_string, builtins::BuiltinPrototype, dep::Dep, entity::Entity,
  value::LiteralValue,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
  fn include(&'a self, _analyzer: &mut Analyzer<'a>) {}

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
    if self.maybe_string() {
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
    EnumeratedProperties {
      known: Default::default(),
      unknown: self.maybe_string().then_some(analyzer.factory.unknown_string),
      dep,
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
    args: ArgumentsValue<'a>,
  ) -> Entity<'a> {
    analyzer.throw_builtin_error("Cannot call non-object");
    if analyzer.config.preserve_exceptions {
      escaped::call(self, analyzer, dep, this, args)
    } else {
      analyzer.factory.never
    }
  }

  fn construct(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    args: ArgumentsValue<'a>,
  ) -> Entity<'a> {
    analyzer.throw_builtin_error("Cannot construct non-object");
    if analyzer.config.preserve_exceptions {
      escaped::construct(self, analyzer, dep, args)
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
    if self.maybe_string() {
      return (vec![], Some(analyzer.factory.unknown), analyzer.dep((self, dep)));
    }
    analyzer.throw_builtin_error("Cannot iterate non-object");
    if analyzer.config.preserve_exceptions {
      self.include(analyzer);
      escaped::iterate(analyzer, dep)
    } else {
      NeverValue.iterate(analyzer, dep)
    }
  }

  fn coerce_string(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.unknown_string
  }

  fn coerce_number(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.unknown
  }

  fn coerce_primitive(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    match self.test_truthy() {
      Some(val) => analyzer.factory.boolean(val),
      None => analyzer.factory.unknown_boolean,
    }
  }

  fn coerce_property_key(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.unknown
  }

  fn coerce_jsx_child(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    if matches!(self, PrimitiveValue::Mixed | PrimitiveValue::String | PrimitiveValue::Number) {
      analyzer.factory.unknown_string
    } else {
      builtin_string!("")
    }
  }
  fn get_keys(
    &'a self,
    _analyzer: &Analyzer<'a>,
    _check_proto: bool,
  ) -> Option<Vec<(bool, Entity<'a>)>> {
    if self.maybe_string() { None } else { Some(vec![]) }
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

  fn is_shared_value(&self) -> bool {
    true
  }

  fn as_cacheable(&self, _analyzer: &Analyzer<'a>) -> Option<Cacheable<'a>> {
    Some(Cacheable::Primitive(*self))
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

  fn maybe_string(&self) -> bool {
    matches!(self, PrimitiveValue::Mixed | PrimitiveValue::String)
  }

  pub fn is_compatible(&self, lit: &LiteralValue) -> bool {
    matches!(
      (self, lit),
      (PrimitiveValue::Mixed, _)
        | (PrimitiveValue::BigInt, LiteralValue::BigInt(_))
        | (PrimitiveValue::Number, LiteralValue::Number(_))
        | (PrimitiveValue::String, LiteralValue::String(_, _))
        | (PrimitiveValue::Boolean, LiteralValue::Boolean(_))
        | (PrimitiveValue::Symbol, LiteralValue::Symbol(_))
    )
  }
}
