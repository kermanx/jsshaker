use crate::{
  Analyzer,
  analyzer::Factory,
  dep::{CustomDepTrait, Dep, DepTrait},
  value::{
    ArgumentsValue, EnumeratedProperties, IteratedElements, LiteralValue, ObjectPrototype,
    TypeofResult, UnionHint, Value, ValueTrait, cacheable::Cacheable,
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

  pub fn consume(&self, analyzer: &mut Analyzer<'a>) {
    analyzer.consume(*self);
  }

  /// Returns true if the entity is completely consumed
  pub fn consume_mangable(&self, analyzer: &mut Analyzer<'a>) -> bool {
    analyzer.consume(self.dep);
    self.value.consume_mangable(analyzer)
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
    self.value.call(analyzer, self.forward_dep(dep, analyzer), this, args)
  }
  pub fn construct(
    &self,
    analyzer: &mut Analyzer<'a>,
    dep: impl DepTrait<'a> + 'a,
    args: ArgumentsValue<'a>,
  ) -> Entity<'a> {
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
  ) -> IteratedElements<'a> {
    self.value.iterate(analyzer, self.forward_dep(dep, analyzer))
  }
  pub fn get_shallow_dep(&self, analyzer: &Analyzer<'a>) -> Dep<'a> {
    if let Some(dep) = self.dep {
      analyzer.dep((dep, self.value.get_shallow_dep(analyzer)))
    } else {
      self.value.get_shallow_dep(analyzer)
    }
  }
  pub fn get_to_string(&self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.forward_value(self.value.get_to_string(analyzer), analyzer)
  }
  pub fn get_to_numeric(&self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.forward_value(self.value.get_to_numeric(analyzer), analyzer)
  }
  pub fn get_to_boolean(&self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.forward_value(self.value.get_to_boolean(analyzer), analyzer)
  }
  pub fn get_to_property_key(&self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.forward_value(self.value.get_to_property_key(analyzer), analyzer)
  }
  pub fn get_to_jsx_child(&self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.forward_value(self.value.get_to_jsx_child(analyzer), analyzer)
  }
  pub fn get_to_literals(&self, analyzer: &Analyzer<'a>) -> Option<Vec<LiteralValue<'a>>> {
    self.value.get_to_literals(analyzer)
  }
  pub fn get_literal(&self, analyzer: &Analyzer<'a>) -> Option<LiteralValue<'a>> {
    self.value.get_literal(analyzer)
  }
  /// Returns vec![(definite, key)]
  pub fn get_own_keys(&self, analyzer: &Analyzer<'a>) -> Option<Vec<(bool, Entity<'a>)>> {
    self.value.get_own_keys(analyzer)
  }
  pub fn get_constructor_prototype(
    &self,
    analyzer: &Analyzer<'a>,
    dep: impl DepTrait<'a> + 'a,
  ) -> Option<(Dep<'a>, ObjectPrototype<'a>, ObjectPrototype<'a>)> {
    self.value.get_constructor_prototype(analyzer, self.forward_dep(dep, analyzer))
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

  pub fn iterate_result_union(
    &self,
    analyzer: &mut Analyzer<'a>,
    dep: impl DepTrait<'a> + 'a,
  ) -> Option<Entity<'a>> {
    self.value.iterate_result_union(analyzer, self.forward_dep(dep, analyzer))
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
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    analyzer.consume(self.value);
    analyzer.consume(self.dep);
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

impl<'a> Factory<'a> {
  pub fn entity_with_dep(&self, value: Value<'a>, dep: Dep<'a>) -> Entity<'a> {
    Entity { value, dep: Some(dep) }
  }

  pub fn computed(&self, entity: Entity<'a>, dep: impl DepTrait<'a> + 'a) -> Entity<'a> {
    Entity {
      value: entity.value,
      dep: if let Some(d) = entity.dep {
        Some((d, dep).uniform(self.allocator))
      } else {
        Some(dep.uniform(self.allocator))
      },
    }
  }
}
