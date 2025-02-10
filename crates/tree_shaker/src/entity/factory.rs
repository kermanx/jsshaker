use crate::{
  builtins::Prototype,
  consumable::{Consumable, LazyConsumable},
  TreeShakeConfig,
};

use super::{
  arguments::ArgumentsEntity, Entity, LiteralEntity, ObjectEntity, PrimitiveEntity,
  PureBuiltinFnEntity, UnknownEntity,
};
use oxc::allocator::Allocator;
use oxc::semantic::{ScopeId, SymbolId};

use std::cell::{Cell, RefCell};
pub struct EntityFactory<'a> {
  pub allocator: &'a Allocator,
  instance_id_counter: Cell<usize>,

  pub r#true: Entity<'a>,
  pub r#false: Entity<'a>,
  pub nan: Entity<'a>,
  pub null: Entity<'a>,
  pub undefined: Entity<'a>,

  pub immutable_unknown: Entity<'a>,

  pub unknown_primitive: Entity<'a>,
  pub unknown_string: Entity<'a>,
  pub unknown_number: Entity<'a>,
  pub unknown_bigint: Entity<'a>,
  pub unknown_boolean: Entity<'a>,
  pub unknown_symbol: Entity<'a>,

  pub pure_fn_returns_unknown: Entity<'a>,
  pub pure_fn_returns_string: Entity<'a>,
  pub pure_fn_returns_number: Entity<'a>,
  pub pure_fn_returns_bigint: Entity<'a>,
  pub pure_fn_returns_boolean: Entity<'a>,
  pub pure_fn_returns_symbol: Entity<'a>,
  pub pure_fn_returns_null: Entity<'a>,
  pub pure_fn_returns_undefined: Entity<'a>,

  pub empty_arguments: Entity<'a>,
  pub unmatched_prototype_property: Entity<'a>,

  pub empty_consumable: Consumable<'a>,
  pub consumed_lazy_consumable: LazyConsumable<'a>,
}

impl<'a> EntityFactory<'a> {
  pub fn new(allocator: &'a Allocator, config: &TreeShakeConfig) -> EntityFactory<'a> {
    let r#true = allocator.alloc(LiteralEntity::Boolean(true));
    let r#false = allocator.alloc(LiteralEntity::Boolean(false));
    let nan = allocator.alloc(LiteralEntity::NaN);
    let null = allocator.alloc(LiteralEntity::Null);
    let undefined = allocator.alloc(LiteralEntity::Undefined);

    let immutable_unknown = allocator.alloc(UnknownEntity::new());
    let unknown_primitive = allocator.alloc(PrimitiveEntity::Mixed);
    let unknown_string = allocator.alloc(PrimitiveEntity::String);
    let unknown_number = allocator.alloc(PrimitiveEntity::Number);
    let unknown_bigint = allocator.alloc(PrimitiveEntity::BigInt);
    let unknown_boolean = allocator.alloc(PrimitiveEntity::Boolean);
    let unknown_symbol = allocator.alloc(PrimitiveEntity::Symbol);

    let pure_fn_returns_unknown = allocator.alloc(PureBuiltinFnEntity::new(|f| f.unknown()));

    let pure_fn_returns_string = allocator.alloc(PureBuiltinFnEntity::new(|f| f.unknown_string));
    let pure_fn_returns_number = allocator.alloc(PureBuiltinFnEntity::new(|f| f.unknown_number));
    let pure_fn_returns_bigint = allocator.alloc(PureBuiltinFnEntity::new(|f| f.unknown_bigint));
    let pure_fn_returns_boolean = allocator.alloc(PureBuiltinFnEntity::new(|f| f.unknown_boolean));
    let pure_fn_returns_symbol = allocator.alloc(PureBuiltinFnEntity::new(|f| f.unknown_symbol));
    let pure_fn_returns_null = allocator.alloc(PureBuiltinFnEntity::new(|f| f.null));
    let pure_fn_returns_undefined = allocator.alloc(PureBuiltinFnEntity::new(|f| f.undefined));

    let empty_arguments = allocator.alloc(ArgumentsEntity::default());
    let unmatched_prototype_property: Entity<'a> =
      if config.unmatched_prototype_property_as_undefined { undefined } else { immutable_unknown };

    let empty_consumable = Consumable(allocator.alloc(()));
    let consumed_lazy_consumable = LazyConsumable(allocator.alloc(RefCell::new(None)));

    EntityFactory {
      allocator,
      instance_id_counter: Cell::new(0),

      r#true,
      r#false,
      nan,
      null,
      undefined,

      immutable_unknown,

      unknown_primitive,
      unknown_string,
      unknown_number,
      unknown_bigint,
      unknown_boolean,
      unknown_symbol,

      pure_fn_returns_unknown,
      pure_fn_returns_string,
      pure_fn_returns_number,
      pure_fn_returns_bigint,
      pure_fn_returns_boolean,
      pure_fn_returns_symbol,
      pure_fn_returns_null,
      pure_fn_returns_undefined,

      empty_arguments,
      unmatched_prototype_property,

      empty_consumable,
      consumed_lazy_consumable,
    }
  }

  pub fn alloc<T>(&self, val: T) -> &'a mut T {
    self.allocator.alloc(val)
  }

  pub fn alloc_instance_id(&self) -> usize {
    let id = self.instance_id_counter.get();
    self.instance_id_counter.set(id + 1);
    id
  }

  pub fn builtin_object(
    &self,
    object_id: SymbolId,
    prototype: &'a Prototype<'a>,
    consumable: bool,
  ) -> &'a mut ObjectEntity<'a> {
    self.alloc(ObjectEntity {
      consumable,
      consumed: Cell::new(false),
      cf_scope: ScopeId::new(0),
      object_id,
      string_keyed: Default::default(),
      unknown_keyed: Default::default(),
      rest: Default::default(),
      prototype,
      mangling_group: None,
    })
  }
}
