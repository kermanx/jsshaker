use std::cell::Cell;

use crate::{
  Analyzer,
  dep::Dep,
  entity::Entity,
  module::ModuleId,
  use_included_flag,
  value::{
    ArgumentsValue, EnumeratedProperties, IteratedElements, LiteralValue, TypeofResult, ValueTrait,
    cacheable::Cacheable, escaped,
  },
};

#[derive(Debug)]
pub struct ModuleObjectValue {
  included: Cell<bool>,
  module: ModuleId,
}

impl<'a> ValueTrait<'a> for ModuleObjectValue {
  fn include(&'a self, analyzer: &mut Analyzer<'a>) {
    use_included_flag!(self);
    analyzer.include_exports(self.module);
  }

  fn unknown_mutate(&'a self, _analyzer: &mut Analyzer<'a>, _dep: Dep<'a>) {
    // Do nothing
  }

  fn get_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    key: Entity<'a>,
  ) -> Entity<'a> {
    let dep = analyzer.dep((dep, key));
    if let Some(key_literals) = key.get_literals(analyzer) {
      let rest_unknown =
        analyzer.does_module_reexport_unknown(self.module, &mut Default::default());
      let mut result = analyzer.factory.vec();
      for &key_literal in &key_literals {
        match key_literal {
          LiteralValue::String(key, _) => {
            if let Some(found) =
              analyzer.get_export_value_by_name(self.module, *key, &mut Default::default())
            {
              result.push(found);
            } else if rest_unknown {
              result.push(analyzer.factory.unknown);
            } else {
              result.push(analyzer.factory.undefined);
            }
          }
          LiteralValue::Symbol(_key, _) => todo!(),
          _ => unreachable!("Invalid property key"),
        }
      }
      analyzer.factory.computed_union(result, (dep, key))
    } else {
      analyzer.factory.computed_unknown((self, dep, key))
    }
  }

  fn set_property(
    &'a self,
    _analyzer: &mut Analyzer<'a>,
    _dep: Dep<'a>,
    _key: Entity<'a>,
    _value: Entity<'a>,
  ) {
    // Do nothing
  }

  fn delete_property(&'a self, _analyzer: &mut Analyzer<'a>, _dep: Dep<'a>, _key: Entity<'a>) {
    // Do nothing
  }

  fn enumerate_properties(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
  ) -> EnumeratedProperties<'a> {
    // TODO: Optimize this
    escaped::enumerate_properties(self, analyzer, dep)
  }

  fn call(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    this: Entity<'a>,
    args: ArgumentsValue<'a>,
  ) -> Entity<'a> {
    escaped::call(self, analyzer, dep, this, args)
  }

  fn construct(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    args: ArgumentsValue<'a>,
  ) -> Entity<'a> {
    escaped::construct(self, analyzer, dep, args)
  }

  fn jsx(&'a self, analyzer: &mut Analyzer<'a>, props: Entity<'a>) -> Entity<'a> {
    escaped::jsx(self, analyzer, props)
  }

  fn r#await(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) -> Entity<'a> {
    self.include(analyzer);
    escaped::r#await(analyzer, dep)
  }

  fn iterate(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) -> IteratedElements<'a> {
    self.include(analyzer);
    escaped::iterate(analyzer, dep)
  }

  fn coerce_string(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    // FIXME: Special methods
    if self.included.get() {
      return escaped::coerce_string(analyzer);
    }
    analyzer.factory.computed_unknown_string(self)
  }

  fn coerce_number(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    // FIXME: Special methods
    if self.included.get() {
      return escaped::coerce_numeric(analyzer);
    }
    analyzer.factory.computed_unknown(self)
  }

  fn coerce_boolean(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.boolean(true)
  }

  fn coerce_property_key(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.coerce_string(analyzer)
  }

  fn coerce_jsx_child(&'a self, _analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.into()
  }

  fn get_own_keys(&'a self, analyzer: &Analyzer<'a>) -> Option<Vec<(bool, Entity<'a>)>> {
    if analyzer.does_module_reexport_unknown(self.module, &mut Default::default()) {
      return None;
    }
    Some(
      analyzer
        .get_exported_keys(self.module, &mut Default::default())
        .into_iter()
        .map(|v| (true, v))
        .collect(),
    )
  }

  fn test_typeof(&self) -> TypeofResult {
    TypeofResult::Object
  }

  fn test_truthy(&self) -> Option<bool> {
    Some(true)
  }

  fn test_nullish(&self) -> Option<bool> {
    Some(false)
  }

  fn as_cacheable(&self, _analyzer: &Analyzer<'a>) -> Option<Cacheable<'a>> {
    Some(Cacheable::ModuleObject(self.module))
  }
}

impl ModuleObjectValue {
  pub fn new(module: ModuleId) -> Self {
    Self { included: Cell::new(false), module }
  }
}
