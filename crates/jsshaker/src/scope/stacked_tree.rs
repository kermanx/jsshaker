use std::fmt::Debug;
use std::hash::Hash;

pub trait Idx: Debug + Clone + Copy + PartialEq + Eq + Hash {
  fn new(depth: usize, parent: usize) -> Self;
  fn depth(self) -> usize;
  fn parent(self) -> usize;
}

#[macro_export]
macro_rules! define_stacked_tree_idx {
  ($v:vis struct $type:ident;) => {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    $v struct $type {
      depth: u32,
      parent: u32,
    }

    impl $crate::scope::stacked_tree::Idx for $type {
      #[inline(always)]
      fn new(depth: usize, parent: usize) -> Self {
        Self { depth: depth as u32, parent: parent as u32 }
      }
      #[inline(always)]
      fn depth(self) -> usize {
        self.depth as usize
      }
      #[inline(always)]
      fn parent(self) -> usize {
        self.parent as usize
      }
    }
  };
}

struct StackedTreeItem<I: Idx, T> {
  id: I,
  id_idx: usize,
  data: T,
}

pub struct StackedTree<I: Idx, T> {
  ids: Vec<I>,
  stack: Vec<StackedTreeItem<I, T>>,
  pub root: I,
}

impl<I: Idx, T> StackedTree<I, T> {
  pub fn new(root_data: T) -> Self {
    let root_id = I::new(0, 0);
    let root_item = StackedTreeItem { id: root_id, id_idx: 0, data: root_data };
    StackedTree { ids: vec![root_id], stack: vec![root_item], root: root_id }
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
    let id = I::new(self.stack.len(), self.stack.last().unwrap().id_idx);
    let id_idx = self.ids.len();
    self.stack.push(StackedTreeItem { id, id_idx, data });
    self.ids.push(id);
    id
  }

  pub fn pop(&mut self) -> T {
    self.stack.pop().unwrap().data
  }

  pub fn depth_to_id(&self, depth: usize) -> I {
    self.stack[depth].id
  }

  fn get_parent(&self, id: I) -> Option<I> {
    if id.depth() == 0 { None } else { Some(self.ids[id.parent()]) }
  }

  pub fn find_lca(&self, another: I) -> (usize, I) {
    let current_depth = self.stack.len() - 1;
    let another_depth = another.depth();
    let min_depth = current_depth.min(another_depth);

    let mut another = another;
    for _ in min_depth..another_depth {
      another = self.get_parent(another).unwrap();
    }
    debug_assert_eq!(min_depth, another.depth());

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
