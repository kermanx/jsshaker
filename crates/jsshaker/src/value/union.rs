use std::{array, cell::Cell, fmt::Debug, iter::Copied, slice};

use oxc::allocator::{self, Allocator};
use rustc_hash::{FxHashMap, FxHashSet};

use super::{
  EnumeratedProperties, IteratedElements, LiteralValue, ObjectPrototype, PropertyKeyValue,
  TypeofResult, UnionHint, ValueTrait,
};
use crate::{
  analyzer::{Analyzer, Factory},
  dep::Dep,
  entity::Entity,
  use_consumed_flag,
};

#[derive(Debug)]
pub struct UnionValue<'a, V: UnionValues<'a> + Debug + 'a> {
  /// Possible values
  pub values: V,
  pub consumed: Cell<bool>,
  pub phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, V: UnionValues<'a> + Debug + 'a> ValueTrait<'a> for UnionValue<'a, V> {
  fn consume(&'a self, analyzer: &mut Analyzer<'a>) {
    use_consumed_flag!(self);

    for value in self.values.iter() {
      value.consume(analyzer);
    }
  }

  fn consume_mangable(&'a self, analyzer: &mut Analyzer<'a>) -> bool {
    if !self.consumed.get() {
      let mut consumed = true;
      for value in self.values.iter() {
        consumed &= value.consume_mangable(analyzer);
      }
      self.consumed.set(consumed);
      consumed
    } else {
      true
    }
  }

  fn unknown_mutate(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) {
    for value in self.values.iter() {
      value.unknown_mutate(analyzer, dep);
    }
  }

  fn get_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    key: Entity<'a>,
  ) -> Entity<'a> {
    let values = analyzer.exec_indeterminately(|analyzer| {
      self.values.map(analyzer.allocator, |v| {
        analyzer.cf_scope_mut().reset_indeterminate();
        v.get_property(analyzer, dep, key)
      })
    });
    analyzer.factory.union(values)
  }

  fn set_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    key: Entity<'a>,
    value: Entity<'a>,
  ) {
    analyzer.exec_indeterminately(|analyzer| {
      for entity in self.values.iter() {
        analyzer.cf_scope_mut().reset_indeterminate();
        entity.set_property(analyzer, dep, key, value)
      }
    });
  }

  fn enumerate_properties(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
  ) -> EnumeratedProperties<'a> {
    let mut total = 0usize;
    let mut known = FxHashMap::<PropertyKeyValue<'a>, (usize, Entity<'a>, Entity<'a>)>::default();
    let mut unknown = analyzer.factory.vec();
    let mut deps = analyzer.factory.vec();
    for entity in self.values.iter() {
      total += 1;
      let enumerated = entity.enumerate_properties(analyzer, dep);
      for (key, (definite, key_v, value)) in enumerated.known {
        known
          .entry(key)
          .and_modify(|(count, key_vs, values)| {
            if definite {
              *count += 1;
            }
            *key_vs = analyzer.factory.union((*key_vs, key_v));
            *values = analyzer.factory.union((*values, value));
          })
          .or_insert((1, key_v, value));
      }
      if let Some(unknown_value) = enumerated.unknown {
        unknown.push(unknown_value);
      }
      deps.push(enumerated.dep);
    }
    EnumeratedProperties {
      known: known
        .into_iter()
        .map(move |(key, (count, key_v, value))| (key, (total == count, key_v, value)))
        .collect(),
      unknown: analyzer.factory.try_union(unknown),
      dep: analyzer.dep(deps),
    }
    // consumed_object::enumerate_properties(self, analyzer, dep)
  }

  fn delete_property(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>, key: Entity<'a>) {
    analyzer.exec_indeterminately(|analyzer| {
      for entity in self.values.iter() {
        analyzer.cf_scope_mut().reset_indeterminate();
        entity.delete_property(analyzer, dep, key);
      }
    })
  }

  fn call(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    this: Entity<'a>,
    args: Entity<'a>,
  ) -> Entity<'a> {
    let values = analyzer.exec_indeterminately(|analyzer| {
      self.values.map(analyzer.allocator, |v| {
        analyzer.cf_scope_mut().reset_indeterminate();
        v.call(analyzer, dep, this, args)
      })
    });
    analyzer.factory.union(values)
  }

  fn construct(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    args: Entity<'a>,
  ) -> Entity<'a> {
    let values = analyzer.exec_indeterminately(|analyzer| {
      self.values.map(analyzer.allocator, |v| {
        analyzer.cf_scope_mut().reset_indeterminate();
        v.construct(analyzer, dep, args)
      })
    });
    analyzer.factory.union(values)
  }

  fn jsx(&'a self, analyzer: &mut Analyzer<'a>, props: Entity<'a>) -> Entity<'a> {
    let values = analyzer.exec_indeterminately(|analyzer| {
      self.values.map(analyzer.allocator, |v| {
        analyzer.cf_scope_mut().reset_indeterminate();
        v.jsx(analyzer, props)
      })
    });
    analyzer.factory.union(values)
  }

  fn r#await(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) -> Entity<'a> {
    let values = analyzer.exec_indeterminately(|analyzer| {
      self.values.map(analyzer.allocator, |v| {
        analyzer.cf_scope_mut().reset_indeterminate();
        v.r#await(analyzer, dep)
      })
    });
    analyzer.factory.union(values)
  }

  fn iterate(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) -> IteratedElements<'a> {
    let mut results = analyzer.factory.vec();
    let mut has_undefined = false;
    analyzer.push_indeterminate_cf_scope();
    for entity in self.values.iter() {
      if let Some(result) = entity.iterate_result_union(analyzer, dep) {
        results.push(result);
      } else {
        has_undefined = true;
      }
    }
    analyzer.pop_cf_scope();
    if has_undefined {
      results.push(analyzer.factory.undefined);
    }
    (vec![], analyzer.factory.try_union(results), analyzer.factory.no_dep)
  }

  fn get_shallow_dep(&'a self, analyzer: &Analyzer<'a>) -> Dep<'a> {
    let mut deps = analyzer.factory.vec();
    for entity in self.values.iter() {
      deps.push(entity.get_shallow_dep(analyzer));
    }
    analyzer.dep(deps)
  }

  fn get_to_string(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    // TODO: dedupe
    let values = self.values.map(analyzer.allocator, |v| v.get_to_string(analyzer));
    analyzer.factory.union(values)
  }

  fn get_to_numeric(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    // TODO: dedupe
    let values = self.values.map(analyzer.allocator, |v| v.get_to_numeric(analyzer));
    analyzer.factory.union(values)
  }

  fn get_to_boolean(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    let values = self.values.map(analyzer.allocator, |v| v.get_to_boolean(analyzer));
    analyzer.factory.union(values)
  }

  fn get_to_property_key(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    let values = self.values.map(analyzer.allocator, |v| v.get_to_property_key(analyzer));
    analyzer.factory.union(values)
  }

  fn get_to_jsx_child(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    let values = self.values.map(analyzer.allocator, |v| v.get_to_jsx_child(analyzer));
    analyzer.factory.union(values)
  }

  fn get_to_literals(&'a self, analyzer: &Analyzer<'a>) -> Option<FxHashSet<LiteralValue<'a>>> {
    let mut iter = self.values.iter();
    let mut result = iter.next().unwrap().get_to_literals(analyzer)?;
    for entity in iter {
      result.extend(entity.get_to_literals(analyzer)?);
    }
    Some(result)
  }

  fn get_own_keys(&'a self, _analyzer: &Analyzer<'a>) -> Option<Vec<(bool, Entity<'a>)>> {
    let mut result = Vec::new();
    for entity in self.values.iter() {
      let keys = entity.get_own_keys(_analyzer)?;
      result.extend(keys.into_iter().map(|(_, key)| (false, key)));
    }
    Some(result)
  }

  fn get_constructor_prototype(
    &'a self,
    _analyzer: &Analyzer<'a>,
    _dep: Dep<'a>,
  ) -> Option<(Dep<'a>, ObjectPrototype<'a>, ObjectPrototype<'a>)> {
    // TODO: Support this
    None
  }

  fn test_typeof(&self) -> TypeofResult {
    let mut result = TypeofResult::_None;
    for entity in self.values.iter() {
      result |= entity.test_typeof();
    }
    result
  }

  fn test_truthy(&self) -> Option<bool> {
    let mut iter = self.values.iter();
    let result = iter.next().unwrap().test_truthy()?;
    for entity in iter {
      if entity.test_truthy()? != result {
        return None;
      }
    }
    Some(result)
  }

  fn test_nullish(&self) -> Option<bool> {
    let mut iter = self.values.iter();
    let result = iter.next().unwrap().test_nullish()?;
    for entity in iter {
      if entity.test_nullish()? != result {
        return None;
      }
    }
    Some(result)
  }
}

pub trait UnionValues<'a> {
  fn len(&self) -> usize;
  type Iter<'b>: Iterator<Item = Entity<'a>>
  where
    Self: 'b,
    'a: 'b;
  fn iter<'b>(&'b self) -> Self::Iter<'b>
  where
    'a: 'b;
  fn map(&self, allocator: &'a Allocator, f: impl FnMut(Entity<'a>) -> Entity<'a>) -> Self;
  fn union(self, factory: &Factory<'a>) -> Entity<'a>;
}

impl<'a> UnionValues<'a> for allocator::Vec<'a, Entity<'a>> {
  fn len(&self) -> usize {
    self.iter().len()
  }
  type Iter<'b>
    = Copied<slice::Iter<'b, Entity<'a>>>
  where
    Self: 'b,
    'a: 'b;
  fn iter<'b>(&'b self) -> Self::Iter<'b>
  where
    'a: 'b,
  {
    self.as_slice().iter().copied()
  }
  fn map(&self, allocator: &'a Allocator, f: impl FnMut(Entity<'a>) -> Entity<'a>) -> Self {
    allocator::Vec::from_iter_in(self.iter().map(f), allocator)
  }
  fn union(self, factory: &Factory<'a>) -> Entity<'a> {
    let mut filtered = factory.vec();
    let mut has_unknown = false;
    for value in &self {
      match value.get_union_hint() {
        UnionHint::Never => continue,
        UnionHint::Unknown => {
          has_unknown = true;
          filtered.push(*value)
        }
        _ => filtered.push(*value),
      }
    }
    if has_unknown {
      return factory.computed_unknown(filtered);
    }
    match filtered.len() {
      0 => factory.never,
      1 => filtered[0],
      2 => factory
        .alloc(UnionValue {
          values: (filtered[0], filtered[1]),
          consumed: Cell::new(false),
          phantom: std::marker::PhantomData,
        })
        .into(),
      _ => factory
        .alloc(UnionValue {
          values: filtered,
          consumed: Cell::new(false),
          phantom: std::marker::PhantomData,
        })
        .into(),
    }
  }
}

impl<'a> UnionValues<'a> for (Entity<'a>, Entity<'a>) {
  fn len(&self) -> usize {
    2
  }
  type Iter<'b>
    = array::IntoIter<Entity<'a>, 2>
  where
    Self: 'b,
    'a: 'b;
  fn iter<'b>(&'b self) -> Self::Iter<'b>
  where
    'a: 'b,
  {
    [self.0, self.1].into_iter()
  }
  fn map(&self, _allocator: &'a Allocator, mut f: impl FnMut(Entity<'a>) -> Entity<'a>) -> Self {
    (f(self.0), f(self.1))
  }
  fn union(self, factory: &Factory<'a>) -> Entity<'a> {
    match (self.0.get_union_hint(), self.1.get_union_hint()) {
      (UnionHint::Never, _) => self.1,
      (_, UnionHint::Never) => self.0,
      (UnionHint::Unknown, _) | (_, UnionHint::Unknown) => factory.computed_unknown(self),
      _ => factory
        .alloc(UnionValue {
          values: self,
          consumed: Cell::new(false),
          phantom: std::marker::PhantomData,
        })
        .into(),
    }
  }
}
