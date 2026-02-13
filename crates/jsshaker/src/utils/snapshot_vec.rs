use std::slice;

use oxc::allocator;

use crate::{Analyzer, dep::DepTrait};

#[derive(Debug)]
pub struct SnapshotVec<'a, T: DepTrait<'a> + Copy + 'a> {
  current: allocator::Vec<'a, T>,
  last_snapshot: usize,
}

impl<'a, T: DepTrait<'a> + Copy + 'a> SnapshotVec<'a, T> {
  pub fn new(allocator: &'a allocator::Allocator) -> Self {
    Self { current: allocator::Vec::new_in(allocator), last_snapshot: 0 }
  }

  pub fn include(&self, analyzer: &mut Analyzer<'a>) {
    for item in &self.current {
      item.include(analyzer);
    }
  }

  pub fn snapshot(&mut self) -> &'a [T] {
    self.last_snapshot = self.current.len();
    unsafe {
      let ptr = self.current.as_ptr();
      let len = self.current.len();
      slice::from_raw_parts(ptr, len)
    }
  }

  pub fn push(&mut self, value: T) {
    self.current.push(value);
  }

  pub fn clear(&mut self, allocator: &'a allocator::Allocator) {
    if self.last_snapshot != 0 {
      self.current = allocator::Vec::new_in(allocator);
      self.last_snapshot = 0;
    } else {
      self.current.clear();
    }
  }

  pub fn truncate(&mut self, len: usize, allocator: &'a allocator::Allocator) {
    if self.last_snapshot > len {
      self.current = allocator::Vec::from_iter_in(self.current[..len].iter().copied(), allocator);
      self.last_snapshot = 0;
    } else {
      self.current.truncate(len);
    }
  }

  pub fn set(&mut self, index: usize, value: T, allocator: &'a allocator::Allocator) {
    if self.last_snapshot > index {
      self.current = allocator::Vec::from_iter_in(self.current.iter().copied(), allocator);
      self.current[index] = value;
      self.last_snapshot = 0;
    } else {
      self.current[index] = value;
    }
  }

  pub fn current_mut(&mut self) -> &mut allocator::Vec<'a, T> {
    debug_assert!(self.last_snapshot == 0, "Cannot get mutable reference after snapshot");
    &mut self.current
  }

  pub fn get(&self, index: usize) -> Option<&T> {
    self.current.get(index)
  }

  pub fn len(&self) -> usize {
    self.current.len()
  }

  pub fn iter(&self) -> slice::Iter<'_, T> {
    self.current.iter()
  }

  pub fn is_empty(&self) -> bool {
    self.current.is_empty()
  }
}
