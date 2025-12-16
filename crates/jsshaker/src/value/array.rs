use std::{
  cell::{Cell, RefCell},
  fmt,
};

use oxc::allocator;
use rustc_hash::FxHashMap;

use super::{
  ArgumentsValue, EnumeratedProperties, IteratedElements, LiteralValue, ObjectId, PropertyKeyValue,
  TypeofResult, ValueTrait, cacheable::Cacheable, consumed_object,
};
use crate::{
  analyzer::{Analyzer, rw_tracking::ReadWriteTarget},
  dep::{CustomDepTrait, Dep, DepCollector, DepVec},
  entity::Entity,
  scope::CfScopeId,
  use_consumed_flag,
};

pub struct ArrayValue<'a> {
  pub consumed: Cell<bool>,
  pub deps: RefCell<DepCollector<'a>>,
  pub cf_scope: CfScopeId,
  pub object_id: ObjectId,
  pub elements: RefCell<allocator::Vec<'a, Entity<'a>>>,
  pub rest: RefCell<allocator::Vec<'a, Entity<'a>>>,
}

impl fmt::Debug for ArrayValue<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("ArrayValue")
      .field("consumed", &self.consumed.get())
      .field("deps", &self.deps.borrow())
      .field("elements", &self.elements.borrow())
      .field("rest", &self.rest.borrow())
      .finish()
  }
}

impl<'a> ValueTrait<'a> for ArrayValue<'a> {
  fn consume(&'a self, analyzer: &mut Analyzer<'a>) {
    use_consumed_flag!(self);

    self.deps.borrow().consume_all(analyzer);
    self.elements.borrow().consume(analyzer);
    self.rest.borrow().consume(analyzer);

    let target_depth = analyzer.find_first_different_cf_scope(self.cf_scope);
    analyzer.track_write(target_depth, ReadWriteTarget::ObjectAll(self.object_id), None);
  }

  fn unknown_mutate(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) {
    if self.consumed.get() {
      return consumed_object::unknown_mutate(analyzer, dep);
    }

    let (is_exhaustive, _, exec_deps) = self.prepare_mutation(analyzer, dep);

    if is_exhaustive {
      self.consume(analyzer);
      return consumed_object::unknown_mutate(analyzer, dep);
    }

    self.deps.borrow_mut().push(analyzer.dep((exec_deps, dep)));
  }

  fn get_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    key: Entity<'a>,
  ) -> Entity<'a> {
    if self.consumed.get() {
      return consumed_object::get_property(self, analyzer, dep, key);
    }

    analyzer.track_read(self.cf_scope, ReadWriteTarget::ObjectAll(self.object_id), None);

    if !self.deps.borrow().is_empty() {
      return analyzer.factory.computed_unknown((self, dep, key));
    }

    let dep = analyzer.dep((self.deps.borrow_mut().collect(analyzer.factory), dep, key));
    if let Some(key_literals) = key.get_to_literals(analyzer) {
      let mut result = analyzer.factory.vec();
      let mut rest_added = false;
      for key_literal in key_literals {
        match key_literal {
          LiteralValue::String(key, _) => {
            if let Ok(index) = key.parse::<usize>() {
              if let Some(element) = self.elements.borrow().get(index) {
                result.push(*element);
              } else if !rest_added {
                rest_added = true;
                result.extend(self.rest.borrow().iter().copied());
                result.push(analyzer.factory.undefined);
              }
            } else if key == "length" {
              result.push(self.get_length().map_or_else(
                || analyzer.factory.computed_unknown_number(&self.rest),
                |length| analyzer.factory.number(length as f64, None),
              ));
            } else if let Some(property) =
              analyzer.builtins.prototypes.array.get_keyed(PropertyKeyValue::String(key))
            {
              result.push(property);
            } else {
              result.push(analyzer.factory.unmatched_prototype_property);
            }
          }
          LiteralValue::Symbol(_key, _) => todo!(),
          _ => unreachable!("Invalid property key"),
        }
      }
      analyzer.factory.computed_union(result, dep)
    } else {
      analyzer.factory.computed_unknown((&self.elements, &self.rest, dep))
    }
  }

  fn set_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    key: Entity<'a>,
    value: Entity<'a>,
  ) {
    if self.consumed.get() {
      return consumed_object::set_property(analyzer, dep, key, value);
    }

    let (is_exhaustive, indeterminate, exec_deps) = self.prepare_mutation(analyzer, dep);

    if is_exhaustive {
      self.consume(analyzer);
      return consumed_object::set_property(analyzer, dep, key, value);
    }

    let mut has_effect = false;
    'known: {
      if !self.deps.borrow().is_empty() {
        break 'known;
      }

      let Some(key_literals) = key.get_to_literals(analyzer) else {
        break 'known;
      };

      let definite = !indeterminate && key_literals.len() == 1;
      let mut rest_added = false;
      for key_literal in key_literals {
        match key_literal {
          LiteralValue::String(key_str, _) => {
            if let Ok(index) = key_str.parse::<usize>() {
              has_effect = true;
              if let Some(element) = self.elements.borrow_mut().get_mut(index) {
                *element = if definite { value } else { analyzer.factory.union((*element, value)) };
              } else if !rest_added {
                rest_added = true;
                self.rest.borrow_mut().push(value);
              }
            } else if key_str == "length" {
              if let Some(length) = value.get_literal(analyzer).and_then(|lit| lit.to_number()) {
                if let Some(length) = length.map(|l| l.0.trunc()) {
                  let length = length as usize;
                  let mut elements = self.elements.borrow_mut();
                  let mut rest = self.rest.borrow_mut();
                  if elements.len() > length {
                    has_effect = true;
                    elements.truncate(length);
                    rest.clear();
                  } else if !rest.is_empty() {
                    has_effect = true;
                    rest.push(analyzer.factory.undefined);
                  } else if elements.len() < length {
                    has_effect = true;
                    for _ in elements.len()..length {
                      elements.push(analyzer.factory.undefined);
                    }
                  }
                } else {
                  analyzer.throw_builtin_error("Invalid array length");
                  has_effect = analyzer.config.preserve_exceptions;
                }
              } else {
                has_effect = true;
              }
            } else {
              break 'known;
            }
          }
          LiteralValue::Symbol(_key, _) => todo!(),
          _ => unreachable!("Invalid property key"),
        }
      }
      if has_effect {
        let mut deps = self.deps.borrow_mut();
        deps.push(analyzer.dep(exec_deps));
      }
      return;
    }

    // Unknown
    let mut deps = self.deps.borrow_mut();
    deps.push(analyzer.dep((exec_deps, key, value)));
  }

  fn enumerate_properties(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
  ) -> EnumeratedProperties<'a> {
    if self.consumed.get() {
      return consumed_object::enumerate_properties(self, analyzer, dep);
    }

    analyzer.track_read(self.cf_scope, ReadWriteTarget::ObjectAll(self.object_id), None);

    if !self.deps.borrow().is_empty() {
      return EnumeratedProperties {
        known: Default::default(),
        unknown: Some(analyzer.factory.unknown),
        dep: analyzer.dep((self, dep)),
      };
    }

    let mut known = FxHashMap::default();
    for (i, element) in self.elements.borrow().iter().enumerate() {
      let i_str = analyzer.allocator.alloc_str(&i.to_string());
      known
        .insert(PropertyKeyValue::String(i_str), (true, analyzer.factory.string(i_str), *element));
    }
    let rest = self.rest.borrow();
    let unknown = (!rest.is_empty()).then(|| {
      analyzer.factory.union(allocator::Vec::from_iter_in(rest.iter().copied(), analyzer.allocator))
    });

    EnumeratedProperties {
      known,
      unknown,
      dep: analyzer.dep((self.deps.borrow_mut().collect(analyzer.factory), dep)),
    }
  }

  fn delete_property(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>, key: Entity<'a>) {
    if self.consumed.get() {
      return consumed_object::delete_property(analyzer, dep, key);
    }

    let (is_exhaustive, _, exec_deps) = self.prepare_mutation(analyzer, dep);

    if is_exhaustive {
      self.consume(analyzer);
      return consumed_object::delete_property(analyzer, dep, key);
    }

    let mut deps = self.deps.borrow_mut();
    deps.push(analyzer.dep((exec_deps, key)));
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
    if self.consumed.get() {
      return consumed_object::r#await(analyzer, dep);
    }
    analyzer.factory.computed(self.into(), dep)
  }

  fn iterate(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) -> IteratedElements<'a> {
    if self.consumed.get() {
      return consumed_object::iterate(analyzer, dep);
    }

    analyzer.track_read(self.cf_scope, ReadWriteTarget::ObjectAll(self.object_id), None);

    if !self.deps.borrow().is_empty() {
      return (vec![], Some(analyzer.factory.unknown), analyzer.dep((self, dep)));
    }

    (
      Vec::from_iter(self.elements.borrow().iter().copied()),
      analyzer.factory.try_union(allocator::Vec::from_iter_in(
        self.rest.borrow().iter().copied(),
        analyzer.allocator,
      )),
      dep,
    )
  }

  fn get_to_string(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    if self.consumed.get() {
      return consumed_object::get_to_string(analyzer);
    }
    analyzer.factory.computed_unknown_string(self)
  }

  fn get_to_numeric(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
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

  fn test_typeof(&self) -> TypeofResult {
    TypeofResult::Object
  }

  fn test_truthy(&self) -> Option<bool> {
    Some(true)
  }

  fn test_nullish(&self) -> Option<bool> {
    Some(false)
  }

  fn as_cachable(&self) -> Option<Cacheable<'a>> {
    None //  Some(Cacheable::Object(self.object_id))
  }
}

impl<'a> ArrayValue<'a> {
  pub fn push_element(&self, element: Entity<'a>) {
    if self.rest.borrow().is_empty() {
      self.elements.borrow_mut().push(element);
    } else {
      self.init_rest(element);
    }
  }

  pub fn init_rest(&self, rest: Entity<'a>) {
    self.rest.borrow_mut().push(rest);
  }

  pub fn get_length(&self) -> Option<usize> {
    if self.rest.borrow().is_empty() { Some(self.elements.borrow().len()) } else { None }
  }

  fn prepare_mutation(
    &self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
  ) -> (bool, bool, DepVec<'a>) {
    let target_depth = analyzer.find_first_different_cf_scope(self.cf_scope);

    let mut is_exhaustive = false;
    let mut indeterminate = false;
    let mut exec_deps = analyzer.factory.vec1(dep);
    for depth in target_depth..analyzer.scoping.cf.stack.len() {
      let scope = analyzer.scoping.cf.get_mut_from_depth(depth);
      is_exhaustive |= scope.is_exhaustive();
      indeterminate |= scope.is_indeterminate();
      if let Some(dep) = scope.deps.try_collect(analyzer.factory) {
        exec_deps.push(dep);
      }
    }

    analyzer.track_write(target_depth, ReadWriteTarget::ObjectAll(self.object_id), None);
    analyzer.request_exhaustive_callbacks(ReadWriteTarget::ObjectAll(self.object_id));

    (is_exhaustive, indeterminate, exec_deps)
  }
}

impl<'a> Analyzer<'a> {
  pub fn new_empty_array(&mut self) -> &'a mut ArrayValue<'a> {
    let cf_scope = self.scoping.cf.current_id();
    let object_id = self.scoping.alloc_object_id();
    self.factory.alloc(ArrayValue {
      consumed: Cell::new(false),
      deps: RefCell::new(DepCollector::new(self.factory.vec())),
      cf_scope,
      object_id,
      elements: RefCell::new(self.factory.vec()),
      rest: RefCell::new(self.factory.vec()),
    })
  }
}
