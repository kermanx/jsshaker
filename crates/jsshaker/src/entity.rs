use crate::{
  Analyzer,
  analyzer::Factory,
  dep::{CustomDepTrait, Dep, DepTrait},
  value::{
    AbstractIterator, ArgumentsValue, EnumeratedProperties, IteratedElements, LiteralValue,
    ObjectPrototype, ObjectValue, TypeofResult, UnionHint, Value, ValueTrait, cacheable::Cacheable,
    literal::PossibleLiterals,
  },
};

#[derive(Debug, Clone, Copy)]
pub struct Entity<'a> {
  value: Value<'a>,
  dep: Option<Dep<'a>>,
}

impl<'a> Entity<'a> {
  fn forward_dep(&self, dep: impl DepTrait<'a> + 'a, analyzer: &Analyzer<'a>) -> Dep<'a> {
    if let Some(d) = self.dep {
      analyzer.factory.dep((d, dep))
    } else {
      dep.uniform(analyzer.factory.allocator)
    }
  }

  fn forward_value(&self, entity: Entity<'a>, analyzer: &Analyzer<'a>) -> Entity<'a> {
    Entity {
      value: entity.value,
      dep: match (self.dep, entity.dep) {
        (Some(d1), Some(d2)) => Some(analyzer.factory.dep((d1, d2))),
        (Some(d), None) | (None, Some(d)) => Some(d),
        (None, None) => None,
      },
    }
  }

  pub fn value_eq(self, other: Self) -> bool {
    std::ptr::eq(self.value, other.value) && !self.value.is_shared_value()
  }

  pub fn exactly_same(self, other: Self) -> bool {
    std::ptr::eq(self.value, other.value)
      && match (self.dep, other.dep) {
        (Some(d1), Some(d2)) => std::ptr::eq(d1.0, d2.0),
        (None, None) => true,
        _ => false,
      }
  }

  pub fn override_dep(self, dep: Dep<'a>) -> Entity<'a> {
    Entity { value: self.value, dep: Some(dep) }
  }

  pub fn include(&self, analyzer: &mut Analyzer<'a>) {
    analyzer.include(self.value);
    analyzer.include(self.dep);
  }

  /// Returns true if the entity is completely included
  pub fn include_mangable(&self, analyzer: &mut Analyzer<'a>) -> bool {
    analyzer.include(self.dep);
    self.value.include_mangable(analyzer)
  }

  pub fn unknown_mutate(&self, analyzer: &mut Analyzer<'a>, dep: impl DepTrait<'a> + 'a) {
    self.value.unknown_mutate(analyzer, self.forward_dep(dep, analyzer));
  }

  pub fn get_property(
    &self,
    analyzer: &mut Analyzer<'a>,
    dep: impl DepTrait<'a> + 'a,
    key: Entity<'a>,
  ) -> Entity<'a> {
    self.value.get_property(analyzer, self.forward_dep(dep, analyzer), key)
  }
  pub fn set_property(
    &self,
    analyzer: &mut Analyzer<'a>,
    dep: impl DepTrait<'a> + 'a,
    key: Entity<'a>,
    value: Entity<'a>,
  ) {
    self.value.set_property(analyzer, self.forward_dep(dep, analyzer), key, value)
  }
  pub fn enumerate_properties(
    &self,
    analyzer: &mut Analyzer<'a>,
    dep: impl DepTrait<'a> + 'a,
  ) -> EnumeratedProperties<'a> {
    self.value.enumerate_properties(analyzer, self.forward_dep(dep, analyzer))
  }
  pub fn delete_property(
    &self,
    analyzer: &mut Analyzer<'a>,
    dep: impl DepTrait<'a> + 'a,
    key: Entity<'a>,
  ) {
    self.value.delete_property(analyzer, self.forward_dep(dep, analyzer), key)
  }
  pub fn call(
    &self,
    analyzer: &mut Analyzer<'a>,
    dep: impl DepTrait<'a> + 'a,
    this: Entity<'a>,
    args: ArgumentsValue<'a>,
  ) -> Entity<'a> {
    let shallow_dep = self.get_shallow_dep(analyzer);
    analyzer.add_callsite_dep(shallow_dep);
    self.value.call(analyzer, self.forward_dep(dep, analyzer), this, args)
  }
  pub fn construct(
    &self,
    analyzer: &mut Analyzer<'a>,
    dep: impl DepTrait<'a> + 'a,
    args: ArgumentsValue<'a>,
  ) -> Entity<'a> {
    let shallow_dep = self.get_shallow_dep(analyzer);
    analyzer.add_callsite_dep(shallow_dep);
    self.value.construct(analyzer, self.forward_dep(dep, analyzer), args)
  }
  pub fn jsx(&self, analyzer: &mut Analyzer<'a>, props: Entity<'a>) -> Entity<'a> {
    self.forward_value(self.value.jsx(analyzer, props), analyzer)
  }
  pub fn r#await(&self, analyzer: &mut Analyzer<'a>, dep: impl DepTrait<'a> + 'a) -> Entity<'a> {
    self.forward_value(self.value.r#await(analyzer, self.forward_dep(dep, analyzer)), analyzer)
  }
  pub fn iterate(
    &self,
    analyzer: &mut Analyzer<'a>,
    dep: impl DepTrait<'a> + 'a,
  ) -> AbstractIterator<'a> {
    self.value.iterate(analyzer, self.forward_dep(dep, analyzer))
  }
  pub fn iterated(
    &self,
    analyzer: &mut Analyzer<'a>,
    dep: impl DepTrait<'a> + 'a,
  ) -> IteratedElements<'a> {
    let (elements, rest, dep, _) = self.iterate(analyzer, dep);
    (elements, rest, dep)
  }

  pub fn coerce_string(&self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.forward_value(self.value.coerce_string(analyzer), analyzer)
  }
  pub fn coerce_number(&self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.forward_value(self.value.coerce_number(analyzer), analyzer)
  }
  pub fn coerce_primitive(&self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.forward_value(self.value.coerce_primitive(analyzer), analyzer)
  }
  pub fn coerce_property_key(&self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.forward_value(self.value.coerce_property_key(analyzer), analyzer)
  }
  pub fn coerce_jsx_child(&self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.forward_value(self.value.coerce_jsx_child(analyzer), analyzer)
  }

  pub fn get_shallow_dep(&self, analyzer: &Analyzer<'a>) -> Dep<'a> {
    match (self.dep, self.value.get_shallow_dep(analyzer)) {
      (Some(d1), Some(d2)) => analyzer.dep((d1, d2)),
      (Some(d), None) | (None, Some(d)) => d,
      (None, None) => analyzer.factory.no_dep,
    }
  }
  pub fn get_literals(&self, analyzer: &Analyzer<'a>) -> Option<PossibleLiterals<'a>> {
    self.value.get_literals(analyzer)
  }
  pub fn get_literal(&self, analyzer: &Analyzer<'a>) -> Option<LiteralValue<'a>> {
    self.value.get_literal(analyzer)
  }
  /// Returns vec![(definite, key)]
  pub fn get_keys(
    &self,
    analyzer: &Analyzer<'a>,
    check_proto: bool,
  ) -> Option<Vec<(bool, Entity<'a>)>> {
    self.value.get_keys(analyzer, check_proto)
  }
  pub fn get_constructor_prototype(
    &self,
    analyzer: &mut Analyzer<'a>,
    dep: impl DepTrait<'a> + 'a,
  ) -> Option<(Dep<'a>, ObjectPrototype<'a>, ObjectPrototype<'a>)> {
    self.value.get_constructor_prototype(analyzer, self.forward_dep(dep, analyzer))
  }
  pub fn as_object(&self) -> Option<&'a ObjectValue<'a>> {
    self.value.as_object()
  }
  pub fn test_typeof(&self) -> TypeofResult {
    self.value.test_typeof()
  }
  pub fn test_truthy(&self) -> Option<bool> {
    self.value.test_truthy()
  }
  pub fn test_nullish(&self) -> Option<bool> {
    self.value.test_nullish()
  }
  pub fn test_is_undefined(&self) -> Option<bool> {
    self.value.test_is_undefined()
  }

  pub fn destruct_as_array(
    &self,
    analyzer: &mut Analyzer<'a>,
    dep: impl DepTrait<'a> + 'a,
    length: usize,
    need_rest: bool,
  ) -> (Vec<Entity<'a>>, Option<Entity<'a>>, Dep<'a>) {
    self.value.destruct_as_array(analyzer, self.forward_dep(dep, analyzer), length, need_rest)
  }

  pub fn call_as_getter(
    &self,
    analyzer: &mut Analyzer<'a>,
    dep: impl DepTrait<'a> + 'a,
    this: Entity<'a>,
  ) -> Entity<'a> {
    self.value.call_as_getter(analyzer, self.forward_dep(dep, analyzer), this)
  }

  pub fn call_as_setter(
    &self,
    analyzer: &mut Analyzer<'a>,
    dep: impl DepTrait<'a> + 'a,
    this: Entity<'a>,
    value: Entity<'a>,
  ) -> Entity<'a> {
    self.value.call_as_setter(analyzer, self.forward_dep(dep, analyzer), this, value)
  }

  pub fn get_union_hint(&self) -> UnionHint {
    self.value.get_union_hint()
  }

  pub fn as_cacheable(&self, analyzer: &Analyzer<'a>) -> Option<Cacheable<'a>> {
    self.value.as_cacheable(analyzer)
  }
}

impl<'a> CustomDepTrait<'a> for Entity<'a> {
  fn include(&self, analyzer: &mut Analyzer<'a>) {
    analyzer.include(self.value);
    analyzer.include(self.dep);
  }
}

impl<'a, T: ValueTrait<'a> + 'a> From<&'a T> for Entity<'a> {
  fn from(value: &'a T) -> Self {
    Entity { value, dep: None }
  }
}
impl<'a, T: ValueTrait<'a> + 'a> From<&'a mut T> for Entity<'a> {
  fn from(value: &'a mut T) -> Self {
    (&*value).into()
  }
}
impl<'a> From<Value<'a>> for Entity<'a> {
  fn from(value: Value<'a>) -> Self {
    Entity { value, dep: None }
  }
}

impl<'a> Factory<'a> {
  pub fn entity_with_dep(&self, value: Value<'a>, dep: Dep<'a>) -> Entity<'a> {
    Entity { value, dep: Some(dep) }
  }

  pub fn computed(&self, entity: Entity<'a>, dep: impl DepTrait<'a> + 'a) -> Entity<'a> {
    Entity {
      value: entity.value,
      dep: if let Some(d) = entity.dep {
        Some(self.dep((d, dep)))
      } else {
        Some(dep.uniform(self.allocator))
      },
    }
  }
}
