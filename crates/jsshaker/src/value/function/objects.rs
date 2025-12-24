use std::cell::Cell;

use crate::{
  Analyzer,
  dep::{Dep, DepCollector},
  mangling::UniquenessGroupId,
  scope::CfScopeId,
  utils::ast::AstKind2,
  value::{
    ObjectProperty, ObjectPropertyValue, ObjectPrototype, ObjectValue, PropertyKeyValue, Value,
  },
};

#[derive(Debug, Clone, Copy)]
enum FnObjectsState<'a> {
  Uninit {
    cf_scope: CfScopeId,
    mangling_group: (Option<UniquenessGroupId>, Option<UniquenessGroupId>),
  },
  Init {
    statics: &'a ObjectValue<'a>,
    prototype: &'a ObjectValue<'a>,
  },
  Consumed,
}

#[derive(Debug)]
pub struct FnObjects<'a>(Cell<FnObjectsState<'a>>);

impl<'a> FnObjects<'a> {
  pub fn new(analyzer: &mut Analyzer<'a>, mangle_node: Option<AstKind2<'a>>) -> Self {
    Self(Cell::new(FnObjectsState::Uninit {
      cf_scope: analyzer.scoping.cf.current_id(),
      mangling_group: if let Some(mangle_node) = mangle_node {
        let (m1, m2) = *analyzer
          .load_data::<Option<(UniquenessGroupId, UniquenessGroupId)>>(mangle_node)
          .get_or_insert_with(|| {
            (analyzer.new_object_mangling_group(), analyzer.new_object_mangling_group())
          });
        (Some(m1), Some(m2))
      } else {
        (None, None)
      },
    }))
  }

  fn is_consumed(&self) -> bool {
    matches!(self.0.get(), FnObjectsState::Consumed)
  }

  fn force_init(
    analyzer: &Analyzer<'a>,
    cf_scope: CfScopeId,
    mangling_group: (Option<UniquenessGroupId>, Option<UniquenessGroupId>),
  ) -> (&'a ObjectValue<'a>, &'a ObjectValue<'a>) {
    let prototype = analyzer.factory.alloc(ObjectValue::new_in(
      analyzer.allocator,
      cf_scope,
      analyzer.scoping.alloc_object_id(),
      ObjectPrototype::Builtin(&analyzer.builtins.prototypes.object),
      mangling_group.0,
    ));
    let statics = analyzer.factory.alloc(ObjectValue::new_in(
      analyzer.allocator,
      cf_scope,
      analyzer.scoping.alloc_object_id(),
      ObjectPrototype::Builtin(&analyzer.builtins.prototypes.function),
      mangling_group.1,
    ));
    statics.keyed.borrow_mut().insert(
      PropertyKeyValue::String("prototype"),
      ObjectProperty {
        definite: true,
        enumerable: false,
        possible_values: analyzer
          .factory
          .vec1(ObjectPropertyValue::Field((&*prototype).into(), false)),
        non_existent: DepCollector::new(analyzer.factory.vec()),
        key: Some(analyzer.factory.builtin_string("prototype")),
        mangling: Some(analyzer.mangler.builtin_atom),
      },
    );
    (statics, prototype)
  }

  pub fn get_objects(&self, analyzer: &Analyzer<'a>) -> (&'a ObjectValue<'a>, &'a ObjectValue<'a>) {
    match self.0.get() {
      FnObjectsState::Uninit { cf_scope, mangling_group } => {
        let (statics, prototype) = Self::force_init(analyzer, cf_scope, mangling_group);
        self.0.set(FnObjectsState::Init { statics, prototype });
        (statics, prototype)
      }
      FnObjectsState::Init { statics, prototype } => (statics, prototype),
      FnObjectsState::Consumed => unreachable!(),
    }
  }

  pub fn get(&self, analyzer: &Analyzer<'a>) -> (Value<'a>, Value<'a>) {
    if self.is_consumed() {
      (analyzer.factory.unknown_value, analyzer.factory.unknown_value)
    } else {
      let (statics, prototype) = self.get_objects(analyzer);
      (statics, prototype)
    }
  }

  pub fn statics(&self, analyzer: &Analyzer<'a>) -> Value<'a> {
    self.get(analyzer).0
  }

  pub fn prototype(&self, analyzer: &Analyzer<'a>) -> Value<'a> {
    self.get(analyzer).1
  }

  pub fn consume(&self, analyzer: &mut Analyzer<'a>) {
    match self.0.replace(FnObjectsState::Consumed) {
      FnObjectsState::Uninit { .. } => {}
      FnObjectsState::Init { statics, prototype } => {
        analyzer.consume(statics);
        analyzer.consume(prototype);
      }
      FnObjectsState::Consumed => {}
    }
  }

  pub fn prototype_object(&self, analyzer: &Analyzer<'a>) -> Option<&'a ObjectValue<'a>> {
    if self.is_consumed() {
      None
    } else {
      let (_, prototype) = self.get_objects(analyzer);
      Some(prototype)
    }
  }

  pub fn get_constructor_prototype(
    &'a self,
    analyzer: &Analyzer<'a>,
    dep: Dep<'a>,
  ) -> (Dep<'a>, ObjectPrototype<'a>, ObjectPrototype<'a>) {
    if self.is_consumed() {
      (dep, ObjectPrototype::Unknown(dep), ObjectPrototype::Unknown(dep))
    } else {
      let (statics, prototype) = self.get_objects(analyzer);
      (dep, ObjectPrototype::Custom(statics), ObjectPrototype::Custom(prototype))
    }
  }
}
