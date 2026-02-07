use std::cell::{Cell, RefCell};

use super::{
  ArgumentsValue, EnumeratedProperties, IteratedElements, TypeofResult, ValueTrait,
  cacheable::Cacheable, escaped,
};
use crate::{
  analyzer::Analyzer,
  dep::{Dep, DepVec},
  entity::Entity,
  use_included_flag,
  value::ObjectPrototype,
};

#[derive(Debug)]
pub struct ReactElementValue<'a> {
  pub included: Cell<bool>,
  pub tag: Entity<'a>,
  pub props: Entity<'a>,
  pub deps: RefCell<DepVec<'a>>,
}

impl<'a> ValueTrait<'a> for ReactElementValue<'a> {
  fn include(&'a self, analyzer: &mut Analyzer<'a>) {
    use_included_flag!(self);

    analyzer.include(&self.deps);

    let tag = self.tag;
    let props = self.props;
    // Is this the best way to handle this?
    let group_id = analyzer.new_object_mangling_group();
    analyzer.exec_included_fn("React_blackbox", move |analyzer| {
      let copied_props = analyzer.new_empty_object(
        ObjectPrototype::Builtin(&analyzer.builtins.prototypes.object),
        Some(group_id),
      );
      copied_props.init_spread(analyzer, analyzer.factory.no_dep, props);
      tag.jsx(analyzer, copied_props.into())
    });
  }

  fn unknown_mutate(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) {
    if self.included.get() {
      return escaped::unknown_mutate(analyzer, dep);
    }

    self.deps.borrow_mut().push(dep);
  }

  fn get_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    key: Entity<'a>,
  ) -> Entity<'a> {
    escaped::get_property(self, analyzer, dep, key)
  }

  fn set_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    key: Entity<'a>,
    value: Entity<'a>,
  ) {
    self.include(analyzer);
    escaped::set_property(analyzer, dep, key, value)
  }

  fn enumerate_properties(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
  ) -> EnumeratedProperties<'a> {
    if analyzer.config.unknown_property_read_side_effects {
      self.include(analyzer);
    }
    escaped::enumerate_properties(self, analyzer, dep)
  }

  fn delete_property(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>, key: Entity<'a>) {
    self.include(analyzer);
    escaped::delete_property(analyzer, dep, key)
  }

  fn call(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    this: Entity<'a>,
    args: ArgumentsValue<'a>,
  ) -> Entity<'a> {
    analyzer.throw_builtin_error("Cannot call a React element");
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
    analyzer.throw_builtin_error("Cannot call a React element");
    if analyzer.config.preserve_exceptions {
      escaped::construct(self, analyzer, dep, args)
    } else {
      analyzer.factory.never
    }
  }

  fn jsx(&'a self, analyzer: &mut Analyzer<'a>, props: Entity<'a>) -> Entity<'a> {
    analyzer.throw_builtin_error("Cannot call a React element");
    if analyzer.config.preserve_exceptions {
      escaped::jsx(self, analyzer, props)
    } else {
      analyzer.factory.never
    }
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
    analyzer.factory.computed_unknown_string(self)
  }

  fn coerce_number(&'a self, _analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.into()
  }

  fn coerce_boolean(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    match self.test_truthy() {
      Some(val) => analyzer.factory.boolean(val),
      None => analyzer.factory.unknown_boolean,
    }
  }

  fn coerce_property_key(&'a self, _analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.into()
  }

  fn coerce_jsx_child(&'a self, _analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.into()
  }

  fn get_constructor_prototype(
    &'a self,
    _analyzer: &mut Analyzer<'a>,
    _dep: Dep<'a>,
  ) -> Option<(Dep<'a>, ObjectPrototype<'a>, ObjectPrototype<'a>)> {
    None
  }

  fn test_typeof(&self) -> TypeofResult {
    TypeofResult::_Unknown
  }

  fn test_truthy(&self) -> Option<bool> {
    None
  }

  fn test_nullish(&self) -> Option<bool> {
    None
  }

  fn as_cacheable(&self, _analyzer: &Analyzer<'a>) -> Option<Cacheable<'a>> {
    None
  }
}

impl<'a> crate::analyzer::Factory<'a> {
  pub fn react_element(&self, tag: Entity<'a>, props: Entity<'a>) -> Entity<'a> {
    self
      .alloc(ReactElementValue {
        included: Cell::new(false),
        tag,
        props,
        deps: RefCell::new(self.vec()),
      })
      .into()
  }
}
