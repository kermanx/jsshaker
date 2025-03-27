use std::mem;

use oxc::allocator;

use super::{Dep, DepTrait};
use crate::analyzer::{Analyzer, Factory};

#[derive(Debug)]
pub struct DepCollector<'a, T: DepTrait<'a> + 'a = Dep<'a>> {
  pub current: allocator::Vec<'a, T>,
  pub node: Option<Dep<'a>>,
}

impl<'a, T: DepTrait<'a> + 'a> DepCollector<'a, T> {
  pub fn new(current: allocator::Vec<'a, T>) -> Self {
    Self { current, node: None }
  }

  pub fn is_empty(&self) -> bool {
    self.current.is_empty() && self.node.is_none()
  }

  pub fn push(&mut self, value: T) {
    self.current.push(value);
  }

  pub fn try_collect(&mut self, factory: &Factory<'a>) -> Option<Dep<'a>> {
    if self.current.is_empty() {
      self.node
    } else {
      let current = mem::replace(&mut self.current, factory.vec());
      let node = Some(if let Some(node) = self.node {
        factory.dep((current, node))
      } else {
        factory.dep(current)
      });
      self.node = node;
      node
    }
  }

  pub fn collect(&mut self, factory: &Factory<'a>) -> Dep<'a> {
    self.try_collect(factory).unwrap_or(factory.no_dep)
  }

  pub fn consume_all(&self, analyzer: &mut Analyzer<'a>) {
    for value in &self.current {
      value.consume(analyzer);
    }

    if let Some(node) = self.node {
      node.consume(analyzer);
    }
  }

  pub fn force_clear(&mut self) {
    self.current.clear();
    self.node = None;
  }

  pub fn may_not_referred(&self) -> bool {
    !self.current.is_empty() || self.node.is_some()
  }
}
