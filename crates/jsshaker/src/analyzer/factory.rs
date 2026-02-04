use std::{
  cell::{Cell, RefCell},
  fmt::Debug,
};

use oxc::allocator::{self, Allocator};

use crate::{
  TreeShakeConfig,
  dep::{CustomDepTrait, Dep, DepTrait, LazyDep, OnceDep},
  entity::Entity,
  mangling::{AlwaysMangableDep, MangleConstraint, ManglingDep},
  scope::CfScopeId,
  utils::CalleeInstanceId,
  value::{
    ArgumentsValue, LiteralValue, PureBuiltinFnValue, logical_result::LogicalResultValue,
    never::NeverValue, primitive::PrimitiveValue, union::UnionValues, unknown::UnknownValue,
  },
};

pub struct Factory<'a> {
  pub allocator: &'a Allocator,
  pub root_cf_scope: Option<CfScopeId>,
  instance_id_counter: Cell<usize>,

  pub r#true: Entity<'a>,
  pub r#false: Entity<'a>,
  pub nan: Entity<'a>,
  pub null: Entity<'a>,
  pub undefined: Entity<'a>,

  pub never: Entity<'a>,
  pub unknown: Entity<'a>,

  pub unknown_primitive: Entity<'a>,
  pub unknown_string: Entity<'a>,
  pub unknown_number: Entity<'a>,
  pub unknown_bigint: Entity<'a>,
  pub unknown_boolean: Entity<'a>,
  pub unknown_symbol: Entity<'a>,

  pub unknown_truthy: Entity<'a>,
  pub unknown_falsy: Entity<'a>,
  pub unknown_nullish: Entity<'a>,
  pub unknown_non_nullish: Entity<'a>,

  pub pure_fn_returns_unknown: Entity<'a>,
  pub pure_fn_returns_string: Entity<'a>,
  pub pure_fn_returns_number: Entity<'a>,
  pub pure_fn_returns_bigint: Entity<'a>,
  pub pure_fn_returns_boolean: Entity<'a>,
  pub pure_fn_returns_symbol: Entity<'a>,
  pub pure_fn_returns_null: Entity<'a>,
  pub pure_fn_returns_undefined: Entity<'a>,

  pub empty_arguments: ArgumentsValue<'a>,
  pub unknown_arguments: ArgumentsValue<'a>,

  pub unmatched_prototype_property: Entity<'a>,

  pub no_dep: Dep<'a>,
  pub consumed_lazy_dep: LazyDep<'a, Dep<'a>>,
}

impl<'a> Factory<'a> {
  pub fn new(allocator: &'a Allocator, config: &TreeShakeConfig) -> Factory<'a> {
    let r#true = allocator.alloc(LiteralValue::Boolean(true)).into();
    let r#false = allocator.alloc(LiteralValue::Boolean(false)).into();
    let nan = allocator.alloc(LiteralValue::Number(f64::NAN.into())).into();
    let null = allocator.alloc(LiteralValue::Null).into();
    let undefined = allocator.alloc(LiteralValue::Undefined).into();

    let never = allocator.alloc(NeverValue).into();
    let immutable_unknown = allocator.alloc(UnknownValue::new()).into();
    let unknown_primitive = allocator.alloc(PrimitiveValue::Mixed).into();
    let unknown_string = allocator.alloc(PrimitiveValue::String).into();
    let unknown_number = allocator.alloc(PrimitiveValue::Number).into();
    let unknown_bigint = allocator.alloc(PrimitiveValue::BigInt).into();
    let unknown_boolean = allocator.alloc(PrimitiveValue::Boolean).into();
    let unknown_symbol = allocator.alloc(PrimitiveValue::Symbol).into();

    let unknown_truthy = allocator
      .alloc(LogicalResultValue { value: immutable_unknown, is_coalesce: false, result: true })
      .into();
    let unknown_falsy = allocator
      .alloc(LogicalResultValue { value: immutable_unknown, is_coalesce: false, result: false })
      .into();
    let unknown_nullish = allocator
      .alloc(LogicalResultValue { value: immutable_unknown, is_coalesce: true, result: true })
      .into();
    let unknown_non_nullish = allocator
      .alloc(LogicalResultValue { value: immutable_unknown, is_coalesce: true, result: false })
      .into();

    let pure_fn_returns_unknown =
      allocator.alloc(PureBuiltinFnValue::new("<PureFn:unknown> ", immutable_unknown)).into();

    let pure_fn_returns_string =
      allocator.alloc(PureBuiltinFnValue::new("<PureFn:string> ", unknown_string)).into();
    let pure_fn_returns_number =
      allocator.alloc(PureBuiltinFnValue::new("<PureFn:number> ", unknown_number)).into();
    let pure_fn_returns_bigint =
      allocator.alloc(PureBuiltinFnValue::new("<PureFn:bigint> ", unknown_bigint)).into();
    let pure_fn_returns_boolean =
      allocator.alloc(PureBuiltinFnValue::new("<PureFn:boolean> ", unknown_boolean)).into();
    let pure_fn_returns_symbol =
      allocator.alloc(PureBuiltinFnValue::new("<PureFn:symbol> ", unknown_symbol)).into();
    let pure_fn_returns_null =
      allocator.alloc(PureBuiltinFnValue::new("<PureFn:null> ", null)).into();
    let pure_fn_returns_undefined =
      allocator.alloc(PureBuiltinFnValue::new("<PureFn:undefined> ", undefined)).into();

    let empty_arguments = ArgumentsValue { elements: &[], rest: None };
    let unknown_arguments = ArgumentsValue { elements: &[], rest: Some(immutable_unknown) };

    let unmatched_prototype_property: Entity<'a> =
      if config.unmatched_prototype_property_as_undefined { undefined } else { immutable_unknown };

    let no_dep = Dep(allocator.alloc(()));
    let consumed_lazy_dep = LazyDep(allocator.alloc(RefCell::new(None)));

    Factory {
      allocator,
      root_cf_scope: None,
      instance_id_counter: Cell::new(0),

      r#true,
      r#false,
      nan,
      null,
      undefined,

      never,
      unknown: immutable_unknown,

      unknown_primitive,
      unknown_string,
      unknown_number,
      unknown_bigint,
      unknown_boolean,
      unknown_symbol,

      unknown_truthy,
      unknown_falsy,
      unknown_nullish,
      unknown_non_nullish,

      pure_fn_returns_unknown,
      pure_fn_returns_string,
      pure_fn_returns_number,
      pure_fn_returns_bigint,
      pure_fn_returns_boolean,
      pure_fn_returns_symbol,
      pure_fn_returns_null,
      pure_fn_returns_undefined,

      empty_arguments,
      unknown_arguments,

      unmatched_prototype_property,

      no_dep,
      consumed_lazy_dep,
    }
  }

  pub fn alloc<T>(&self, val: T) -> &'a mut T {
    self.allocator.alloc(val)
  }

  pub fn vec<T>(&self) -> allocator::Vec<'a, T> {
    allocator::Vec::new_in(self.allocator)
  }

  pub fn vec1<T>(&self, v: T) -> allocator::Vec<'a, T> {
    let mut vec = allocator::Vec::with_capacity_in(1, self.allocator);
    vec.push(v);
    vec
  }

  pub fn alloc_instance_id(&self) -> CalleeInstanceId {
    let id = self.instance_id_counter.get();
    self.instance_id_counter.set(id + 1);
    CalleeInstanceId::from_usize(id)
  }

  pub fn dep_no_once(&self, dep: impl CustomDepTrait<'a> + 'a) -> Dep<'a> {
    Dep(self.alloc(dep))
  }

  pub fn dep_once(&self, dep: impl CustomDepTrait<'a> + 'a) -> Dep<'a> {
    self.dep_no_once(OnceDep::new(dep))
  }

  pub fn dep(&self, dep: impl CustomDepTrait<'a> + 'a) -> Dep<'a> {
    self.dep_once(dep)
  }

  pub fn dep_from_vec<T: DepTrait<'a> + 'a>(&self, deps: allocator::Vec<'a, T>) -> Dep<'a> {
    match deps.len() {
      0 => self.no_dep,
      1 => deps.into_iter().next().unwrap().uniform(self.allocator),
      _ => self.dep(deps),
    }
  }

  pub fn optional_computed(
    &self,
    val: Entity<'a>,
    dep: Option<impl DepTrait<'a> + 'a>,
  ) -> Entity<'a> {
    match dep {
      Some(dep) => self.computed(val, dep),
      None => val,
    }
  }

  pub fn try_union<V: UnionValues<'a> + Debug + 'a>(&self, values: V) -> Option<Entity<'a>> {
    match values.len() {
      0 => None,
      1 => Some(values.iter().next().unwrap()),
      _ => Some(values.union(self)),
    }
  }

  pub fn union<V: UnionValues<'a> + Debug + 'a>(&self, values: V) -> Entity<'a> {
    self.try_union(values).unwrap()
  }

  pub fn optional_union(
    &self,
    entity: Entity<'a>,
    entity_option: Option<Entity<'a>>,
  ) -> Entity<'a> {
    if let Some(entity_option) = entity_option {
      self.union((entity, entity_option))
    } else {
      entity
    }
  }

  pub fn computed_union<V: UnionValues<'a> + Debug + 'a, T: DepTrait<'a> + 'a>(
    &self,
    values: V,
    dep: T,
  ) -> Entity<'a> {
    self.computed(self.union(values), dep)
  }

  pub fn computed_unknown(&self, dep: impl DepTrait<'a> + 'a) -> Entity<'a> {
    self.computed(self.unknown, dep)
  }

  pub fn lazy_dep<T: DepTrait<'a> + 'a>(&self, deps: allocator::Vec<'a, T>) -> LazyDep<'a, T> {
    LazyDep(self.alloc(RefCell::new(Some(deps))))
  }

  pub fn mangable(
    &self,
    val: Entity<'a>,
    deps: (Entity<'a>, Entity<'a>),
    constraint: MangleConstraint<'a>,
  ) -> Entity<'a> {
    self.computed(val, ManglingDep { deps, constraint })
  }

  pub fn always_mangable_dep(&self, dep: Entity<'a>) -> Dep<'a> {
    self.dep(AlwaysMangableDep { dep })
  }
}

macro_rules! unknown_entity_ctors {
  ($($name:ident -> $var:ident,)*) => {
    $(
      #[allow(unused)]
      pub fn $name(&self, dep: impl DepTrait<'a> + 'a) -> Entity<'a> {
        self.computed(self.$var, dep)
      }
    )*
  };
}

impl<'a> Factory<'a> {
  unknown_entity_ctors! {
    computed_unknown_primitive -> unknown_primitive,
    computed_unknown_boolean -> unknown_boolean,
    computed_unknown_number -> unknown_number,
    computed_unknown_string -> unknown_string,
    computed_unknown_bigint -> unknown_bigint,
    computed_unknown_symbol -> unknown_symbol,
  }
}

#[macro_export]
macro_rules! builtin_atom {
  ($s:expr) => {{
    const S: oxc::span::Atom = oxc::span::Atom::new_const($s);
    (&S)
  }};
}

#[macro_export]
macro_rules! builtin_string {
  ($s:expr) => {{
    $crate::entity::Entity::from(&$crate::value::literal::LiteralValue::String(
      $crate::builtin_atom!($s),
      Some($crate::mangling::BUILTIN_ATOM),
    ))
  }};
}
