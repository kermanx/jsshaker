mod delete;
mod enumerate;
mod get;
mod init;
mod property;
mod set;

use std::{
  cell::{Cell, RefCell},
  fmt::Debug,
};

use oxc::allocator;
pub use property::{ObjectProperty, ObjectPropertyValue};

use super::{
  AbstractIterator, ArgumentsValue, EnumeratedProperties, PropertyKeyValue, TypeofResult,
  ValueTrait, cacheable::Cacheable, escaped,
};
use crate::{
  analyzer::{Analyzer, rw_tracking::ReadWriteTarget},
  builtin_atom, builtin_string,
  builtins::BuiltinPrototype,
  define_ptr_idx,
  dep::{Dep, DepAtom, DepCollector},
  entity::Entity,
  mangling::{BUILTIN_ATOM, MangleAtom, UniquenessGroupId, is_literal_mangable},
  scope::CfScopeId,
  use_included_flag,
  utils::ast::AstKind2,
  value::{UnionHint, literal::PossibleLiterals},
};

#[derive(Debug, Clone, Copy)]
pub enum ObjectPrototype<'a> {
  ImplicitOrNull,
  Builtin(&'a BuiltinPrototype<'a>),
  Custom(&'a ObjectValue<'a>),
  Unknown(Dep<'a>),
}

impl<'a> ObjectPrototype<'a> {
  fn include(&self, analyzer: &mut Analyzer<'a>) {
    match self {
      ObjectPrototype::ImplicitOrNull => {}
      ObjectPrototype::Builtin(_prototype) => {}
      ObjectPrototype::Custom(object) => object.include_as_prototype(analyzer),
      ObjectPrototype::Unknown(dep) => analyzer.include(*dep),
    }
  }
}

define_ptr_idx! {
  pub struct ObjectId for ObjectValue<'a>;
}

#[derive(Debug)]
pub struct ObjectValue<'a> {
  /// A built-in object is usually immutable
  pub immutable: bool,
  pub is_builtin_function: bool,

  pub included: Cell<bool>,
  pub included_as_prototype: Cell<bool>,
  /// Where the object is created
  pub cf_scope: CfScopeId,
  pub prototype: Cell<ObjectPrototype<'a>>,
  /// `None` if not mangable
  /// `Some(None)` if mangable at the beginning, but disabled later
  pub mangling_group: Cell<Option<UniquenessGroupId>>,

  /// Properties keyed by known string
  pub keyed: RefCell<allocator::HashMap<'a, PropertyKeyValue<'a>, ObjectProperty<'a>>>,
  /// Properties keyed by unknown value
  pub unknown: RefCell<ObjectProperty<'a>>,
  /// Properties keyed by unknown value, but not included in `keyed`
  pub rest: Option<allocator::Box<'a, RefCell<ObjectProperty<'a>>>>,
}

impl<'a> ValueTrait<'a> for ObjectValue<'a> {
  fn include(&'a self, analyzer: &mut Analyzer<'a>) {
    if self.immutable {
      return;
    }

    use_included_flag!(self);

    self.include_as_prototype(analyzer);

    self.keyed.borrow_mut().clear();
    self.unknown.replace_with(|_| ObjectProperty::new_in(analyzer.allocator));

    let target_depth = analyzer.find_first_different_cf_scope(self.cf_scope);
    analyzer.track_write(target_depth, ReadWriteTarget::ObjectAll(self.object_id()), None);
    analyzer.request_exhaustive_callbacks(ReadWriteTarget::ObjectAll(self.object_id()));
  }

  fn unknown_mutate(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) {
    if self.immutable {
      return;
    }

    if self.included_as_prototype.get() {
      return escaped::unknown_mutate(analyzer, dep);
    }

    self.add_extra_dep(dep);

    let target_depth = analyzer.find_first_different_cf_scope(self.cf_scope);
    let should_include =
      analyzer.track_write(target_depth, ReadWriteTarget::ObjectAll(self.object_id()), None);
    analyzer.request_exhaustive_callbacks(ReadWriteTarget::ObjectAll(self.object_id()));
    if should_include.is_some() {
      self.include(analyzer);
    } else {
      let mut unknown = self.unknown.borrow_mut();
      if let Some(ObjectPropertyValue::Property(Some(v1), Some(v2))) =
        unknown.possible_values.last()
        && v1.exactly_same(*v2)
        && v2.exactly_same(analyzer.factory.unknown)
      {
      } else {
        unknown.possible_values.push(ObjectPropertyValue::Property(
          Some(analyzer.factory.unknown),
          Some(analyzer.factory.unknown),
        ));
      }
    }
  }

  fn get_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    key: Entity<'a>,
  ) -> Entity<'a> {
    self.get_property(analyzer, self, dep, key)
  }

  fn set_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    key: Entity<'a>,
    value: Entity<'a>,
  ) {
    self.set_property(analyzer, dep, key, value);
  }

  fn enumerate_properties(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
  ) -> EnumeratedProperties<'a> {
    self.enumerate_properties(analyzer, self, dep)
  }

  fn delete_property(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>, key: Entity<'a>) {
    self.delete_property(analyzer, dep, key);
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

  fn iterate(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) -> AbstractIterator<'a> {
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

  fn coerce_primitive(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.r#true
  }

  fn coerce_property_key(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.coerce_string(analyzer)
  }

  fn coerce_jsx_child(&'a self, _analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.into()
  }

  fn get_keys(
    &'a self,
    analyzer: &Analyzer<'a>,
    check_proto: bool,
  ) -> Option<Vec<(bool, Entity<'a>)>> {
    if self.included.get() {
      return None;
    }

    let mut unknown = self.unknown.borrow_mut();
    if self.rest.is_some() || !unknown.possible_values.is_empty() {
      return None;
    }

    let mut keys = Vec::new();

    if check_proto {
      match self.prototype.get() {
        ObjectPrototype::Custom(proto) => keys = proto.get_keys(analyzer, true)?,
        ObjectPrototype::Builtin(_) => {}
        ObjectPrototype::Unknown(_) => return None,
        ObjectPrototype::ImplicitOrNull => {}
      }
    }

    for (key, property) in self.keyed.borrow_mut().iter_mut() {
      let key_entity = property.key.unwrap_or_else(|| {
        if let PropertyKeyValue::String(key) = key {
          analyzer.factory.string(*key, property.mangling)
        } else {
          todo!()
        }
      });
      let key_entity = if property.non_existent.is_empty() {
        key_entity
      } else {
        analyzer.factory.computed(key_entity, property.non_existent.collect(analyzer.factory))
      };
      let key_entity = if unknown.non_existent.is_empty() {
        key_entity
      } else {
        analyzer.factory.computed(key_entity, unknown.non_existent.collect(analyzer.factory))
      };
      let key_entity = analyzer.factory.computed(key_entity, {
        let mut deps = analyzer.factory.vec();
        for value in &property.possible_values {
          deps.push(match value {
            ObjectPropertyValue::Consumed(value, _) => *value,
            ObjectPropertyValue::Field(value, _) => *value,
            ObjectPropertyValue::Property(Some(getter), _) => *getter,
            ObjectPropertyValue::Property(None, _) => analyzer.factory.undefined,
          })
        }
        deps
      });
      keys.push((property.definite, key_entity));
    }

    Some(keys)
  }

  fn as_object(&'a self) -> Option<&'a ObjectValue<'a>> {
    Some(self)
  }

  fn test_typeof(&self) -> TypeofResult {
    if self.is_builtin_function { TypeofResult::Function } else { TypeofResult::Object }
  }

  fn test_truthy(&self) -> Option<bool> {
    Some(true)
  }

  fn test_nullish(&self) -> Option<bool> {
    Some(false)
  }

  fn as_cacheable(&self, _analyzer: &Analyzer<'a>) -> Option<Cacheable<'a>> {
    Some(Cacheable::Object(self.object_id()))
  }

  fn get_union_hint(&self) -> UnionHint {
    UnionHint::Object
  }
}

impl<'a> ObjectValue<'a> {
  pub fn object_id(&self) -> ObjectId {
    ObjectId::from_ref(self)
  }

  fn include_as_prototype(&self, analyzer: &mut Analyzer<'a>) {
    if self.included_as_prototype.replace(true) {
      return;
    }

    self.disable_mangling(analyzer);

    self.prototype.get().include(analyzer);

    let mut suspended = analyzer.factory.vec();
    for property in self.keyed.borrow().values() {
      property.include(analyzer, &mut suspended);
    }
    self.unknown.borrow().include(analyzer, &mut suspended);
    analyzer.include(suspended);
  }

  pub fn is_mangable(&self) -> bool {
    self.mangling_group.get().is_some()
  }

  fn check_mangable(&self, analyzer: &mut Analyzer<'a>, literals: &PossibleLiterals<'a>) -> bool {
    if self.is_mangable() {
      if is_literal_mangable(literals) {
        true
      } else {
        self.disable_mangling(analyzer);
        false
      }
    } else {
      false
    }
  }

  fn disable_mangling(&self, analyzer: &mut Analyzer<'a>) {
    if let Some(group) = self.mangling_group.replace(None) {
      analyzer.mangler.mark_uniqueness_group_non_mangable(group);
    }
  }

  fn add_to_mangling_group(&self, analyzer: &mut Analyzer<'a>, key_atom: MangleAtom) {
    analyzer.mangler.add_to_uniqueness_group(self.mangling_group.get().unwrap(), key_atom);
  }

  pub fn set_prototype(&self, prototype: ObjectPrototype<'a>) {
    self.prototype.set(prototype);
  }

  pub fn add_extra_dep(&self, dep: Dep<'a>) {
    self.unknown.borrow_mut().non_existent.push(dep);
  }
}

impl<'a> Analyzer<'a> {
  pub fn new_empty_object(
    &mut self,
    prototype: ObjectPrototype<'a>,
    mangling_group: Option<UniquenessGroupId>,
  ) -> &'a mut ObjectValue<'a> {
    self.allocator.alloc(ObjectValue {
      immutable: false,
      is_builtin_function: false,
      included: Cell::new(false),
      included_as_prototype: Cell::new(false),
      // deps: Default::default(),
      cf_scope: self.scoping.cf.current_id(),
      keyed: RefCell::new(allocator::HashMap::new_in(self.allocator)),
      unknown: RefCell::new(ObjectProperty::new_in(self.allocator)),
      rest: None,
      prototype: Cell::new(prototype),
      mangling_group: Cell::new(mangling_group),
    })
  }

  pub fn new_function_object(
    &mut self,
    mangle_node: Option<AstKind2<'a>>,
  ) -> (&'a ObjectValue<'a>, &'a ObjectValue<'a>) {
    let mangling_group = if let Some(mangle_node) = mangle_node {
      let (m1, m2) = *self
        .load_data::<Option<(UniquenessGroupId, UniquenessGroupId)>>(mangle_node)
        .get_or_insert_with(|| {
          (self.new_object_mangling_group(), self.new_object_mangling_group())
        });
      (Some(m1), Some(m2))
    } else {
      (None, None)
    };
    let prototype = self.new_empty_object(
      ObjectPrototype::Builtin(&self.builtins.prototypes.object),
      mangling_group.0,
    );
    let statics = self.new_empty_object(
      ObjectPrototype::Builtin(&self.builtins.prototypes.function),
      mangling_group.1,
    );
    statics.keyed.borrow_mut().insert(
      PropertyKeyValue::String(builtin_atom!("prototype")),
      ObjectProperty {
        definite: true,
        enumerable: false,
        possible_values: self.factory.vec1(ObjectPropertyValue::Field((&*prototype).into(), false)),
        non_existent: DepCollector::new(self.factory.vec()),
        key: Some(builtin_string!("prototype")),
        mangling: Some(BUILTIN_ATOM),
      },
    );
    (statics, prototype)
  }

  pub fn new_object_mangling_group(&mut self) -> UniquenessGroupId {
    self.mangler.uniqueness_groups.push(Default::default())
  }

  pub fn use_mangable_plain_object(
    &mut self,
    dep_id: impl Into<DepAtom>,
  ) -> &'a mut ObjectValue<'a> {
    let mangling_group = self
      .load_data::<Option<UniquenessGroupId>>(dep_id)
      .get_or_insert_with(|| self.new_object_mangling_group());
    self.new_empty_object(
      ObjectPrototype::Builtin(&self.builtins.prototypes.object),
      Some(*mangling_group),
    )
  }
}

impl<'a> crate::analyzer::Factory<'a> {
  pub fn builtin_object(&self, prototype: ObjectPrototype<'a>) -> &'a mut ObjectValue<'a> {
    self.alloc(ObjectValue {
      immutable: true,
      is_builtin_function: false,
      included: Cell::new(false),
      included_as_prototype: Cell::new(false),
      cf_scope: self.root_cf_scope.unwrap(),
      keyed: allocator::HashMap::new_in(self.allocator).into(),
      unknown: ObjectProperty::new_in(self.allocator).into(),
      rest: Default::default(),
      prototype: Cell::new(prototype),
      mangling_group: Cell::new(None),
    })
  }
}
