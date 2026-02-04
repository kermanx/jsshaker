use oxc::allocator::Allocator;

use crate::utils::box_bump::{BoxBump, Idx};

pub struct StackedTreeId<I: Idx> {
  depth: usize,
  parent: Option<I>,
}

#[macro_export]
macro_rules! define_stacked_tree_idx {
  ($v:vis struct $type:ident;) => {
    $crate::define_box_bump_idx! {
      $v struct $type for $crate::scope::stacked_tree::StackedTreeId<$type>;
    }
  };
}

struct StackedTreeItem<I: Idx, T> {
  id: I,
  data: T,
}

pub struct StackedTree<'a, I: Idx, T> {
  ids: BoxBump<'a, I, StackedTreeId<I>>,
  stack: Vec<StackedTreeItem<I, T>>,
  pub root: I,
}

impl<'a, I: Idx, T> StackedTree<'a, I, T> {
  pub fn new(allocator: &'a Allocator, root_data: T) -> Self {
    let ids = BoxBump::new(allocator);
    let root_id = ids.alloc(StackedTreeId { depth: 0, parent: None });
    let root_item = StackedTreeItem { id: root_id, data: root_data };
    StackedTree { ids, stack: vec![root_item], root: root_id }
  }

  pub fn current_id(&self) -> I {
    self.stack.last().unwrap().id
  }

  pub fn current_depth(&self) -> usize {
    self.stack.len() - 1
  }

  pub fn current_data(&self) -> &T {
    &self.stack.last().unwrap().data
  }

  pub fn current_data_mut(&mut self) -> &mut T {
    &mut self.stack.last_mut().unwrap().data
  }

  pub fn data_at(&self, depth: usize) -> &T {
    &self.stack[depth].data
  }

  pub fn data_at_mut(&mut self, depth: usize) -> &mut T {
    &mut self.stack[depth].data
  }

  pub fn stack_len(&self) -> usize {
    self.stack.len()
  }

  pub fn iter_stack(&self) -> impl DoubleEndedIterator<Item = &T> + ExactSizeIterator {
    self.stack.iter().map(|item| &item.data)
  }

  pub fn iter_stack_mut(&mut self) -> impl DoubleEndedIterator<Item = &mut T> + ExactSizeIterator {
    self.stack.iter_mut().map(|item| &mut item.data)
  }

  pub fn push(&mut self, data: T) -> I {
    let id = self.ids.alloc(StackedTreeId {
      depth: self.stack.len(),
      parent: Some(self.stack.last().unwrap().id),
    });
    self.stack.push(StackedTreeItem { id, data });
    id
  }

  pub fn pop(&mut self) -> T {
    self.stack.pop().unwrap().data
  }

  pub fn depth_to_id(&self, depth: usize) -> I {
    self.stack[depth].id
  }

  fn get_depth(&self, id: I) -> usize {
    self.ids.get(id).depth
  }

  fn get_parent(&self, id: I) -> Option<I> {
    self.ids.get(id).parent
  }

  pub fn find_lca(&self, another: I) -> (usize, I) {
    let current_depth = self.stack.len() - 1;
    let another_depth = self.get_depth(another);
    let min_depth = current_depth.min(another_depth);

    let mut another = another;
    for _ in min_depth..another_depth {
      another = self.get_parent(another).unwrap();
    }
    debug_assert_eq!(min_depth, self.get_depth(another));

    let mut depth = min_depth;
    loop {
      if self.stack[depth].id == another {
        break;
      }
      depth -= 1;
      another = self.get_parent(another).unwrap();
    }

    debug_assert_eq!(self.stack[depth].id, another);
    (depth, another)
  }
}
