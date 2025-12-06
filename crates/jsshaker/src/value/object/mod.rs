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
use oxc_index::define_index_type;
pub use property::{ObjectProperty, ObjectPropertyValue};
use rustc_hash::FxHashSet;

use super::{
  ArgumentsValue, EnumeratedProperties, IteratedElements, LiteralValue, PropertyKeyValue,
  TypeofResult, ValueTrait, cachable::Cachable, consumed_object,
};
use crate::{
  analyzer::Analyzer,
  builtins::BuiltinPrototype,
  dep::{Dep, DepAtom, DepCollector},
  entity::Entity,
  mangling::{MangleAtom, UniquenessGroupId, is_literal_mangable},
  scope::{CfScopeId, rw_tracking::ReadWriteTarget},
  use_consumed_flag,
  utils::ast::AstKind2,
};

type ObjectManglingGroupId<'a> = &'a Cell<Option<UniquenessGroupId>>;

#[derive(Debug, Clone, Copy)]
pub enum ObjectPrototype<'a> {
  ImplicitOrNull,
  Builtin(&'a BuiltinPrototype<'a>),
  Custom(&'a ObjectValue<'a>),
  Unknown(Dep<'a>),
}

impl<'a> ObjectPrototype<'a> {
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    match self {
      ObjectPrototype::ImplicitOrNull => {}
      ObjectPrototype::Builtin(_prototype) => {}
      ObjectPrototype::Custom(object) => object.consume_as_prototype(analyzer),
      ObjectPrototype::Unknown(dep) => analyzer.consume(*dep),
    }
  }
}

define_index_type! {
  pub struct ObjectId = u32;
}

pub struct ObjectValue<'a> {
  /// A built-in object is usually non-consumable
  pub consumable: bool,
  pub consumed: Cell<bool>,
  pub consumed_as_prototype: Cell<bool>,
  /// Where the object is created
  pub cf_scope: CfScopeId,
  pub object_id: ObjectId,
  pub prototype: Cell<ObjectPrototype<'a>>,
  /// `None` if not mangable
  /// `Some(None)` if mangable at the beginning, but disabled later
  pub mangling_group: Cell<Option<ObjectManglingGroupId<'a>>>,

  /// Properties keyed by known string
  pub keyed: RefCell<allocator::HashMap<'a, PropertyKeyValue<'a>, ObjectProperty<'a>>>,
  /// Properties keyed by unknown value
  pub unknown: RefCell<ObjectProperty<'a>>,
  /// Properties keyed by unknown value, but not included in `keyed`
  pub rest: Option<allocator::Box<'a, RefCell<ObjectProperty<'a>>>>,
}

impl Debug for ObjectValue<'_> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("ObjectValue").finish()
  }
}

impl<'a> ValueTrait<'a> for ObjectValue<'a> {
  fn consume(&'a self, analyzer: &mut Analyzer<'a>) {
    if !self.consumable {
      return;
    }

    use_consumed_flag!(self);

    self.consume_as_prototype(analyzer);

    self.keyed.borrow_mut().clear();
    self.unknown.replace_with(|_| ObjectProperty::new_in(analyzer.allocator));

    let target_depth = analyzer.find_first_different_cf_scope(self.cf_scope);
    analyzer.track_write(ReadWriteTarget::ObjectAll(self.object_id), target_depth);
  }

  fn unknown_mutate(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) {
    if self.consumed.get() {
      return consumed_object::unknown_mutate(analyzer, dep);
    }

    self.add_extra_dep(dep);

    let target_depth = analyzer.find_first_different_cf_scope(self.cf_scope);
    let (should_consume, _) =
      analyzer.track_write(ReadWriteTarget::ObjectAll(self.object_id), target_depth);
    if should_consume {
      self.consume(analyzer);
    }
  }

  fn get_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    key: Entity<'a>,
  ) -> Entity<'a> {
    self.get_property(analyzer, dep, key)
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
    self.enumerate_properties(analyzer, dep)
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
    consumed_object::call(self, analyzer, dep, this, args)
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
    consumed_object::jsx(self, analyzer, props)
  }

  fn r#await(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) -> Entity<'a> {
    self.consume(analyzer);
    consumed_object::r#await(analyzer, dep)
  }

  fn iterate(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) -> IteratedElements<'a> {
    self.consume(analyzer);
    consumed_object::iterate(analyzer, dep)
  }

  fn get_to_string(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    // FIXME: Special methods
    if self.consumed.get() {
      return consumed_object::get_to_string(analyzer);
    }
    analyzer.factory.computed_unknown_string(self)
  }

  fn get_to_numeric(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    // FIXME: Special methods
    if self.consumed.get() {
      return consumed_object::get_to_numeric(analyzer);
    }
    analyzer.factory.computed_unknown(self)
  }

  fn get_to_boolean(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.boolean(true)
  }

  fn get_to_property_key(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.get_to_string(analyzer)
  }

  fn get_to_jsx_child(&'a self, _analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.into()
  }

  fn get_own_keys(&'a self, analyzer: &Analyzer<'a>) -> Option<Vec<(bool, Entity<'a>)>> {
    let mut unknown = self.unknown.borrow_mut();
    if self.consumed.get() || self.rest.is_some() || !unknown.possible_values.is_empty() {
      return None;
    }

    let mut keys = Vec::new();
    for (key, property) in self.keyed.borrow_mut().iter_mut() {
      let key_entity = property.key.unwrap_or_else(|| {
        if let PropertyKeyValue::String(key) = key { analyzer.factory.string(key) } else { todo!() }
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

  fn test_typeof(&self) -> TypeofResult {
    TypeofResult::Object
  }

  fn test_truthy(&self) -> Option<bool> {
    Some(true)
  }

  fn test_nullish(&self) -> Option<bool> {
    Some(false)
  }

  fn as_cachable(&self) -> Option<Cachable<'a>> {
    None //  Some(Cachable::Object(self.object_id))
  }
}

impl<'a> ObjectValue<'a> {
  fn consume_as_prototype(&self, analyzer: &mut Analyzer<'a>) {
    if self.consumed_as_prototype.replace(true) {
      return;
    }

    self.disable_mangling(analyzer);

    self.prototype.get().consume(analyzer);

    let mut suspended = analyzer.factory.vec();
    for property in self.keyed.borrow().values() {
      property.consume(analyzer, &mut suspended);
    }
    self.unknown.borrow().consume(analyzer, &mut suspended);
    analyzer.consume(suspended);
  }

  pub fn is_mangable(&self) -> bool {
    self.mangling_group.get().is_some_and(|group| group.get().is_some())
  }

  fn check_mangable(
    &self,
    analyzer: &mut Analyzer<'a>,
    literals: &FxHashSet<LiteralValue>,
  ) -> bool {
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
    if let Some(group) = self.mangling_group.get()
      && let Some(group) = group.replace(None)
    {
      analyzer.mangler.mark_uniqueness_group_non_mangable(group);
    }
  }

  fn add_to_mangling_group(&self, analyzer: &mut Analyzer<'a>, key_atom: MangleAtom) {
    analyzer
      .mangler
      .add_to_uniqueness_group(self.mangling_group.get().unwrap().get().unwrap(), key_atom);
  }

  pub fn set_prototype(&self, prototype: ObjectPrototype<'a>) {
    self.prototype.set(prototype);
    match prototype {
      ObjectPrototype::Custom(s) => self.mangling_group.set(s.mangling_group.get()),
      ObjectPrototype::Builtin(_) | ObjectPrototype::Unknown(_) => self.mangling_group.set(None),
      ObjectPrototype::ImplicitOrNull => {}
    }
  }

  pub fn add_extra_dep(&self, dep: Dep<'a>) {
    self.unknown.borrow_mut().non_existent.push(dep);
  }
}

impl<'a> Analyzer<'a> {
  pub fn new_empty_object(
    &mut self,
    prototype: ObjectPrototype<'a>,
    mangling_group: Option<ObjectManglingGroupId<'a>>,
  ) -> &'a mut ObjectValue<'a> {
    self.allocator.alloc(ObjectValue {
      consumable: true,
      consumed: Cell::new(false),
      consumed_as_prototype: Cell::new(false),
      // deps: Default::default(),
      cf_scope: self.scoping.cf.current_id(),
      object_id: self.scoping.alloc_object_id(),
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
        .load_data::<Option<(ObjectManglingGroupId, ObjectManglingGroupId)>>(mangle_node)
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
      PropertyKeyValue::String("prototype"),
      ObjectProperty {
        definite: true,
        enumerable: false,
        possible_values: self.factory.vec1(ObjectPropertyValue::Field((&*prototype).into(), false)),
        non_existent: DepCollector::new(self.factory.vec()),
        key: Some(self.factory.string("prototype")),
        mangling: Some(self.mangler.builtin_atom),
      },
    );
    (statics, prototype)
  }

  pub fn new_object_mangling_group(&mut self) -> ObjectManglingGroupId<'a> {
    self.allocator.alloc(Cell::new(Some(self.mangler.uniqueness_groups.push(Default::default()))))
  }

  pub fn use_mangable_plain_object(
    &mut self,
    dep_id: impl Into<DepAtom>,
  ) -> &'a mut ObjectValue<'a> {
    let mangling_group = self
      .load_data::<Option<ObjectManglingGroupId>>(dep_id)
      .get_or_insert_with(|| self.new_object_mangling_group());
    self.new_empty_object(
      ObjectPrototype::Builtin(&self.builtins.prototypes.object),
      Some(*mangling_group),
    )
  }
}
