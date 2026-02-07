use oxc::allocator;

use crate::{
  analyzer::Analyzer,
  dep::{CustomDepTrait, Dep},
  entity::Entity,
};

#[derive(Debug, Clone, Copy)]
pub struct ArgumentsValue<'a> {
  pub elements: &'a [Entity<'a>],
  pub rest: Option<Entity<'a>>,
}

impl<'a> CustomDepTrait<'a> for ArgumentsValue<'a> {
  fn include(&self, analyzer: &mut Analyzer<'a>) {
    analyzer.include(self.elements);
    analyzer.include(self.rest);
  }
}

impl<'a> ArgumentsValue<'a> {
  pub fn unknown_mutate(&self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) {
    for entity in self.elements {
      entity.unknown_mutate(analyzer, dep);
    }
    if let Some(rest) = self.rest {
      rest.unknown_mutate(analyzer, dep);
    }
  }

  pub fn get_last_shallow_dep(&self, analyzer: &mut Analyzer<'a>) -> Dep<'a> {
    if let Some(e) = self.rest {
      return e.get_shallow_dep(analyzer);
    }
    if let Some(e) = self.elements.last() {
      return e.get_shallow_dep(analyzer);
    }
    analyzer.factory.no_dep
  }

  pub fn get(&self, analyzer: &mut Analyzer<'a>, nth: usize) -> Entity<'a> {
    if nth < self.elements.len() {
      self.elements[nth]
    } else if let Some(rest) = self.rest {
      rest
    } else {
      analyzer.factory.undefined
    }
  }

  pub fn split_at(
    &self,
    analyzer: &mut Analyzer<'a>,
    mid: usize,
  ) -> (Vec<Entity<'a>>, ArgumentsValue<'a>) {
    if mid <= self.elements.len() {
      let (left, right) = self.elements.split_at(mid);
      (left.to_vec(), ArgumentsValue { elements: right, rest: self.rest })
    } else {
      let mut left = self.elements.to_vec();
      left.resize(mid, analyzer.factory.undefined);
      (left, ArgumentsValue { elements: &[], rest: self.rest })
    }
  }

  pub fn from_value(
    analyzer: &mut Analyzer<'a>,
    value: Entity<'a>,
    dep: Dep<'a>,
  ) -> ArgumentsValue<'a> {
    let (elements, rest, dep) = value.iterate(analyzer, dep);
    let elements = allocator::Vec::from_iter_in(
      elements.into_iter().map(|e| analyzer.factory.computed(e, dep)),
      analyzer.allocator,
    );
    let rest = rest.map(|r| analyzer.factory.computed(r, dep));
    ArgumentsValue { elements: analyzer.factory.alloc(elements), rest }
  }

  pub fn from_concatenate(
    analyzer: &mut Analyzer<'a>,
    args1: ArgumentsValue<'a>,
    args2: ArgumentsValue<'a>,
  ) -> ArgumentsValue<'a> {
    if args2.elements.is_empty() {
      ArgumentsValue {
        elements: args1.elements,
        rest: match (args1.rest, args2.rest) {
          (Some(r1), Some(r2)) => Some(analyzer.factory.union((r1, r2))),
          (Some(r1), None) => Some(r1),
          (None, Some(r2)) => Some(r2),
          (None, None) => None,
        },
      }
    } else if let Some(rest1) = args1.rest {
      let mut rest = analyzer.factory.vec();
      rest.push(rest1);
      for elem in args2.elements {
        rest.push(*elem);
      }
      if let Some(rest2) = args2.rest {
        rest.push(rest2);
      }
      ArgumentsValue { elements: args1.elements, rest: Some(analyzer.factory.union(rest)) }
    } else {
      let mut new_elements = analyzer.factory.vec();
      new_elements.extend_from_slice(args1.elements);
      new_elements.extend_from_slice(args2.elements);
      ArgumentsValue { elements: new_elements.into_bump_slice(), rest: args2.rest }
    }
  }
}

impl<'a> crate::analyzer::Factory<'a> {
  pub fn arguments(
    &self,
    elements: &'a [Entity<'a>],
    rest: Option<Entity<'a>>,
  ) -> ArgumentsValue<'a> {
    ArgumentsValue { elements, rest }
  }
}
