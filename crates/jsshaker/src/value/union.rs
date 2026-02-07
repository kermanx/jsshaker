use std::{array, cell::Cell, fmt::Debug, iter::Copied, slice};

use oxc::allocator::{self, Allocator};
use rustc_hash::{FxHashMap, FxHashSet};

use super::{
  ArgumentsValue, EnumeratedProperties, IteratedElements, LiteralValue, ObjectPrototype,
  PropertyKeyValue, TypeofResult, UnionHint, ValueTrait, cacheable::Cacheable,
};
use crate::{
  analyzer::{Analyzer, Factory},
  dep::Dep,
  entity::Entity,
  use_consumed_flag,
  value::{ObjectValue, literal::PossibleLiterals},
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
    let values = analyzer.exec_non_det(|analyzer| {
      self.values.map(analyzer.allocator, |v| {
        analyzer.cf_scope_mut().reset_non_det();
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
    analyzer.exec_non_det(|analyzer| {
      for entity in self.values.iter() {
        analyzer.cf_scope_mut().reset_non_det();
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
  }

  fn delete_property(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>, key: Entity<'a>) {
    analyzer.exec_non_det(|analyzer| {
      for entity in self.values.iter() {
        analyzer.cf_scope_mut().reset_non_det();
        entity.delete_property(analyzer, dep, key);
      }
    })
  }

  fn call(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    this: Entity<'a>,
    args: ArgumentsValue<'a>,
  ) -> Entity<'a> {
    let values = analyzer.exec_non_det(|analyzer| {
      self.values.map(analyzer.allocator, |v| {
        analyzer.cf_scope_mut().reset_non_det();
        v.call(analyzer, dep, this, args)
      })
    });
    analyzer.factory.union(values)
  }

  fn construct(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    args: ArgumentsValue<'a>,
  ) -> Entity<'a> {
    let values = analyzer.exec_non_det(|analyzer| {
      self.values.map(analyzer.allocator, |v| {
        analyzer.cf_scope_mut().reset_non_det();
        v.construct(analyzer, dep, args)
      })
    });
    analyzer.factory.union(values)
  }

  fn jsx(&'a self, analyzer: &mut Analyzer<'a>, props: Entity<'a>) -> Entity<'a> {
    let values = analyzer.exec_non_det(|analyzer| {
      self.values.map(analyzer.allocator, |v| {
        analyzer.cf_scope_mut().reset_non_det();
        v.jsx(analyzer, props)
      })
    });
    analyzer.factory.union(values)
  }

  fn r#await(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) -> Entity<'a> {
    let values = analyzer.exec_non_det(|analyzer| {
      self.values.map(analyzer.allocator, |v| {
        analyzer.cf_scope_mut().reset_non_det();
        v.r#await(analyzer, dep)
      })
    });
    analyzer.factory.union(values)
  }

  fn iterate(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) -> IteratedElements<'a> {
    let mut elements = Vec::new();
    let mut max_elements = usize::MAX;
    let mut rest = analyzer.factory.vec();
    let mut deps = analyzer.factory.vec();

    analyzer.push_non_det_cf_scope();
    for entity in self.values.iter() {
      analyzer.cf_scope_mut().reset_non_det();
      let (e, r, d) = entity.iterate(analyzer, dep);
      let n = max_elements.min(e.len());
      if r.is_some() {
        max_elements = n;
      }
      if elements.len() < n {
        elements.resize_with(n, || analyzer.factory.vec());
      }
      for (i, el) in e[..n].iter().copied().enumerate() {
        elements[i].push(el);
      }
      for el in e.iter().copied().skip(max_elements).chain(r) {
        rest.push(el);
      }
      deps.push(d);
    }
    if elements.len() > max_elements {
      for e in elements.drain(max_elements..) {
        rest.extend(e);
      }
    }
    analyzer.pop_cf_scope();

    (
      elements
        .into_iter()
        .map(|mut e| {
          if e.len() < self.values.len() {
            e.push(analyzer.factory.undefined);
          }
          analyzer.factory.union(e)
        })
        .collect(),
      analyzer.factory.try_union(rest),
      analyzer.factory.dep(deps),
    )
  }

  fn get_shallow_dep(&'a self, analyzer: &Analyzer<'a>) -> Option<Dep<'a>> {
    let mut deps = analyzer.factory.vec();
    for entity in self.values.iter() {
      deps.push(entity.get_shallow_dep(analyzer));
    }
    if deps.is_empty() { None } else { Some(analyzer.dep(deps)) }
  }

  fn coerce_string(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    // TODO: dedupe
    let values = self.values.map(analyzer.allocator, |v| v.coerce_string(analyzer));
    analyzer.factory.union(values)
  }

  fn coerce_number(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    // TODO: dedupe
    let values = self.values.map(analyzer.allocator, |v| v.coerce_number(analyzer));
    analyzer.factory.union(values)
  }

  fn coerce_boolean(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    let values = self.values.map(analyzer.allocator, |v| v.coerce_boolean(analyzer));
    analyzer.factory.union(values)
  }

  fn coerce_property_key(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    let values = self.values.map(analyzer.allocator, |v| v.coerce_property_key(analyzer));
    analyzer.factory.union(values)
  }

  fn coerce_jsx_child(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    let values = self.values.map(analyzer.allocator, |v| v.coerce_jsx_child(analyzer));
    analyzer.factory.union(values)
  }

  fn get_literals(&'a self, analyzer: &Analyzer<'a>) -> Option<PossibleLiterals<'a>> {
    let mut result = FxHashSet::default();
    for entity in self.values.iter() {
      result.extend(entity.get_literals(analyzer)?.into_iter());
    }
    Some(if result.len() == 1 {
      PossibleLiterals::Single(result.into_iter().next().unwrap())
    } else {
      PossibleLiterals::Multiple(result.into_iter().collect())
    })
  }

  fn get_literal(&'a self, analyzer: &Analyzer<'a>) -> Option<LiteralValue<'a>> {
    let mut iter = self.values.iter();
    let result = iter.next()?.get_literal(analyzer)?;
    for entity in iter {
      let lit = entity.get_literal(analyzer)?;
      if lit != result {
        return None;
      }
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
    _analyzer: &mut Analyzer<'a>,
    _dep: Dep<'a>,
  ) -> Option<(Dep<'a>, ObjectPrototype<'a>, ObjectPrototype<'a>)> {
    // TODO: Support this
    None
  }
  fn as_object(&'a self) -> Option<&'a ObjectValue<'a>> {
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

  fn as_cacheable(&self, analyzer: &Analyzer<'a>) -> Option<Cacheable<'a>> {
    let mut result = Cacheable::Never;
    for value in self.values.iter() {
      result = result.union(analyzer.allocator, value.as_cacheable(analyzer)?);
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
        UnionHint::Object => {
          filtered = self;
          has_unknown = false;
          break;
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
    let no_merge = || {
      factory
        .alloc(UnionValue {
          values: self,
          consumed: Cell::new(false),
          phantom: std::marker::PhantomData,
        })
        .into()
    };
    match (self.0.get_union_hint(), self.1.get_union_hint()) {
      (UnionHint::Object, _) | (_, UnionHint::Object) => no_merge(),
      (UnionHint::Never, _) => self.1,
      (_, UnionHint::Never) => self.0,
      (UnionHint::Unknown, _) | (_, UnionHint::Unknown) => factory.computed_unknown(self),
      _ => no_merge(),
    }
  }
}
