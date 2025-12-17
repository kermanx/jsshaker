use std::mem;

use crate::utils::box_bump::{BoxBump, Idx};
use oxc::allocator::Allocator;

struct NodeInfo<I, T> {
  data: T,
  parent: Option<I>,
}

pub struct LinkedTree<'a, I: Idx, T> {
  nodes: BoxBump<'a, I, NodeInfo<I, T>>,
  top: Option<I>,
}

impl<'a, I: Idx, T> LinkedTree<'a, I, T> {
  pub fn new_in(allocator: &'a Allocator) -> Self {
    LinkedTree { nodes: BoxBump::new(allocator), top: None }
  }

  pub fn top(&self) -> Option<I> {
    self.top
  }

  pub fn get(&self, id: I) -> &T {
    &self.nodes.get(id).data
  }

  pub fn get_mut(&mut self, id: I) -> &mut T {
    &mut self.nodes.get_mut(id).data
  }

  pub fn get_current(&self) -> &T {
    self.get(self.top.unwrap())
  }

  pub fn get_current_mut(&mut self) -> &mut T {
    self.get_mut(self.top.unwrap())
  }

  pub fn get_parent(&self, id: I) -> Option<I> {
    self.nodes.get(id).parent
  }

  pub fn push(&mut self, data: T) -> I {
    let id = self.nodes.alloc(NodeInfo { data, parent: self.top });
    self.top = Some(id);
    id
  }

  pub fn pop(&mut self) -> I {
    let current = self.top.unwrap();
    self.top = self.get_parent(current);
    current
  }

  pub fn replace_top(&mut self, new_top: Option<I>) -> Option<I> {
    mem::replace(&mut self.top, new_top)
  }

  pub fn iter_rev<'t>(&'t self) -> RevIterator<'t, 'a, I, T> {
    RevIterator { tree: self, current: self.top }
  }
}

pub struct RevIterator<'t, 'a, I: Idx, T> {
  tree: &'t LinkedTree<'a, I, T>,
  current: Option<I>,
}

impl<'t, 'a, I: Idx, T> Iterator for RevIterator<'t, 'a, I, T> {
  type Item = &'t T;

  fn next(&mut self) -> Option<Self::Item> {
    if let Some(current) = self.current {
      self.current = self.tree.get_parent(current);
      Some(self.tree.get(current))
    } else {
      None
    }
  }
}
