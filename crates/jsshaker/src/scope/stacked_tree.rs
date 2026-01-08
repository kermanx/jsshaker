use crate::utils::box_bump::{BoxBump, Idx};
use oxc::allocator::Allocator;

pub struct NodeInfo<I, T> {
  pub data: T,
  depth: usize,
  parent: Option<I>,
}

pub struct StackedTree<'a, I: Idx, T> {
  pub nodes: BoxBump<'a, I, NodeInfo<I, T>>,
  pub stack: Vec<I>,
}

impl<'a, I: Idx, T> StackedTree<'a, I, T> {
  pub fn new_in(allocator: &'a Allocator) -> Self {
    StackedTree { nodes: BoxBump::new(allocator), stack: vec![] }
  }

  pub fn current_id(&self) -> I {
    *self.stack.last().unwrap()
  }

  pub fn current_depth(&self) -> usize {
    self.stack.len() - 1
  }

  pub fn get(&self, id: I) -> &T {
    &self.nodes.get(id).data
  }

  pub fn get_mut(&mut self, id: I) -> &mut T {
    &mut self.nodes.get_mut(id).data
  }

  pub fn get_mut_from_depth(&mut self, depth: usize) -> &mut T {
    let id = self.stack[depth];
    self.get_mut(id)
  }

  pub fn get_current(&self) -> &T {
    self.get(*self.stack.last().unwrap())
  }

  pub fn get_current_mut(&mut self) -> &mut T {
    self.get_mut(*self.stack.last().unwrap())
  }

  pub fn iter_stack(&self) -> impl DoubleEndedIterator<Item = &T> + ExactSizeIterator<Item = &T> {
    self.stack.iter().map(move |id| self.get(*id))
  }

  fn get_parent(&self, id: I) -> Option<I> {
    self.nodes.get(id).parent
  }

  pub fn push(&mut self, data: T) -> I {
    let id = self.nodes.alloc(NodeInfo {
      data,
      depth: self.stack.len(),
      parent: self.stack.last().copied(),
    });
    self.stack.push(id);
    id
  }

  pub fn pop(&mut self) -> I {
    self.stack.pop().unwrap()
  }

  pub fn find_lca(&self, another: I) -> (usize, I) {
    let another_info = self.nodes.get(another);
    let current_depth = self.stack.len() - 1;
    let another_depth = another_info.depth;
    let min_depth = current_depth.min(another_depth);

    let mut another = another;
    for _ in 0..(another_depth - min_depth) {
      another = self.get_parent(another).unwrap();
    }

    let mut current_idx = min_depth;
    loop {
      if self.stack[current_idx] == another {
        break;
      }
      current_idx -= 1;
      another = self.get_parent(another).unwrap();
    }

    assert_eq!(self.stack[current_idx], another);
    (current_idx, another)
  }

  pub fn replace_stack(&mut self, stack: Vec<I>) -> Vec<I> {
    std::mem::replace(&mut self.stack, stack)
  }
}
