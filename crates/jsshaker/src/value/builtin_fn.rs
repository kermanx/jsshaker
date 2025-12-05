use std::{cell::Cell, fmt::Debug};

use super::{
  EnumeratedProperties, IteratedElements, ObjectPrototype, ObjectValue, TypeofResult, ValueTrait,
  arguments::ArgumentsValue, consumed_object, never::NeverValue,
};
use crate::{
  analyzer::{Analyzer, Factory},
  dep::Dep,
  entity::Entity,
  use_consumed_flag,
};

trait BuiltinFnImpl<'a>: Debug {
  fn name(&self) -> &'static str;
  fn object(&self) -> Option<&'a ObjectValue<'a>> {
    None
  }
  fn call_impl(
    &self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    this: Entity<'a>,
    args: ArgumentsValue<'a>,
  ) -> Entity<'a>;
  fn consume(&'a self, _analyzer: &mut Analyzer<'a>) {}
}

impl<'a, T: BuiltinFnImpl<'a>> ValueTrait<'a> for T {
  fn consume(&'a self, analyzer: &mut Analyzer<'a>) {
    if let Some(object) = self.object() {
      object.consume(analyzer);
    }
    self.consume(analyzer);
  }

  fn unknown_mutate(&'a self, _analyzer: &mut Analyzer<'a>, _dep: Dep<'a>) {
    // No effect
  }

  fn get_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    key: Entity<'a>,
  ) -> Entity<'a> {
    if let Some(object) = self.object() {
      object.get_property(analyzer, dep, key)
    } else {
      analyzer.builtins.prototypes.function.get_property(analyzer, self.into(), key, dep)
    }
  }

  fn set_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    key: Entity<'a>,
    value: Entity<'a>,
  ) {
    if let Some(object) = self.object() {
      object.set_property(analyzer, dep, key, value)
    } else {
      analyzer.add_diagnostic(
        format!(
          "Should not set property of builtin function `{}`, it may cause unexpected tree-shaking behavior",
          self.name()
        )
    );
      consumed_object::set_property(analyzer, dep, key, value)
    }
  }

  fn delete_property(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>, key: Entity<'a>) {
    if let Some(object) = self.object() {
      object.delete_property(analyzer, dep, key)
    } else {
      analyzer.add_diagnostic("Should not delete property of builtin function, it may cause unexpected tree-shaking behavior");
      consumed_object::delete_property(analyzer, dep, key)
    }
  }

  fn enumerate_properties(
    &'a self,
    _analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
  ) -> EnumeratedProperties<'a> {
    EnumeratedProperties { known: Default::default(), unknown: None, dep }
  }

  fn call(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    this: Entity<'a>,
    args: ArgumentsValue<'a>,
  ) -> Entity<'a> {
    #[cfg(feature = "flame")]
    let _scope_guard = flame::start_guard(self.name());
    self.call_impl(analyzer, dep, this, args)
  }

  fn construct(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    args: ArgumentsValue<'a>,
  ) -> Entity<'a> {
    consumed_object::construct(self, analyzer, dep, args)
  }

  fn jsx(&'a self, analyzer: &mut Analyzer<'a>, props: Entity<'a>) -> Entity<'a> {
    self.call_impl(
      analyzer,
      analyzer.factory.no_dep,
      analyzer.factory.unknown,
      analyzer.factory.arguments(analyzer.factory.alloc([props]), None),
    )
  }

  fn r#await(&'a self, _analyzer: &mut Analyzer<'a>, _dep: Dep<'a>) -> Entity<'a> {
    self.into()
  }

  fn iterate(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) -> IteratedElements<'a> {
    analyzer.throw_builtin_error("Cannot iterate over function");
    if analyzer.config.preserve_exceptions {
      consumed_object::iterate(analyzer, dep)
    } else {
      NeverValue.iterate(analyzer, dep)
    }
  }

  fn get_to_string(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.computed_unknown_string(self)
  }

  fn get_to_numeric(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.nan
  }

  fn get_to_boolean(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.boolean(true)
  }

  fn get_to_property_key(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.get_to_string(analyzer)
  }

  fn get_to_jsx_child(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    // TODO: analyzer.thrown_builtin_error("Functions are not valid JSX children");
    analyzer.factory.string("")
  }

  fn get_constructor_prototype(
    &'a self,
    _analyzer: &Analyzer<'a>,
    _dep: Dep<'a>,
  ) -> Option<(Dep<'a>, ObjectPrototype<'a>, ObjectPrototype<'a>)> {
    None
  }

  fn test_typeof(&self) -> TypeofResult {
    TypeofResult::Function
  }

  fn test_truthy(&self) -> Option<bool> {
    Some(true)
  }

  fn test_nullish(&self) -> Option<bool> {
    Some(false)
  }
}

pub trait BuiltinFnImplementation<'a>:
  Fn(&mut Analyzer<'a>, Dep<'a>, Entity<'a>, ArgumentsValue<'a>) -> Entity<'a>
{
}
impl<'a, T: Fn(&mut Analyzer<'a>, Dep<'a>, Entity<'a>, ArgumentsValue<'a>) -> Entity<'a>>
  BuiltinFnImplementation<'a> for T
{
}

#[derive(Clone)]
pub struct ImplementedBuiltinFnValue<'a, F: BuiltinFnImplementation<'a> + 'a> {
  pub name: &'static str,
  pub implementation: F,
  pub object: Option<&'a ObjectValue<'a>>,
  pub consumed: Cell<bool>,
}

impl<'a, F: BuiltinFnImplementation<'a> + 'a> Debug for ImplementedBuiltinFnValue<'a, F> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("ImplementedBuiltinFnValue").finish()
  }
}

impl<'a, F: BuiltinFnImplementation<'a> + 'a> BuiltinFnImpl<'a>
  for ImplementedBuiltinFnValue<'a, F>
{
  fn name(&self) -> &'static str {
    self.name
  }
  fn object(&self) -> Option<&'a ObjectValue<'a>> {
    self.object
  }
  fn call_impl(
    &self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    this: Entity<'a>,
    args: ArgumentsValue<'a>,
  ) -> Entity<'a> {
    (self.implementation)(analyzer, dep, this, args)
  }
  fn consume(&'a self, analyzer: &mut Analyzer<'a>) {
    use_consumed_flag!(self);

    let name = self.name;

    analyzer.exec_consumed_fn(name, move |analyzer| {
      self.call_impl(
        analyzer,
        analyzer.factory.no_dep,
        analyzer.factory.unknown,
        analyzer.factory.unknown_arguments,
      )
    });
  }
}

impl<'a> Analyzer<'a> {
  pub fn dynamic_implemented_builtin<F: BuiltinFnImplementation<'a> + 'a>(
    &mut self,
    name: &'static str,
    implementation: F,
  ) -> Entity<'a> {
    self
      .factory
      .alloc(ImplementedBuiltinFnValue {
        name,
        implementation,
        object: Some(self.new_function_object(None).0),
        consumed: Cell::new(false),
      })
      .into()
  }
}

#[derive(Debug, Clone)]
pub struct PureBuiltinFnValue<'a> {
  return_value: fn(&Factory<'a>) -> Entity<'a>,
}

impl<'a> BuiltinFnImpl<'a> for PureBuiltinFnValue<'a> {
  fn name(&self) -> &'static str {
    "<PureBuiltin>"
  }
  fn call_impl(
    &self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    this: Entity<'a>,
    args: ArgumentsValue<'a>,
  ) -> Entity<'a> {
    let ret_val = (self.return_value)(analyzer.factory);
    let dep = analyzer.dep((dep, this, args));
    this.unknown_mutate(analyzer, dep);
    analyzer.factory.computed(ret_val, dep)
  }
}

impl<'a> PureBuiltinFnValue<'a> {
  pub fn new(return_value: fn(&Factory<'a>) -> Entity<'a>) -> Self {
    Self { return_value }
  }
}
