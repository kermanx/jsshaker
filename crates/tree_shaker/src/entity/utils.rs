use core::slice;
use std::{array, iter::Copied};

#[macro_export]
macro_rules! use_consumed_flag {
  ($self: expr) => {
    if $self.consumed.replace(true) {
      return;
    }
  };
}

pub trait UnionLike<'a, T: 'a + Copy> {
  fn len(&self) -> usize;
  type Iter<'b>: Iterator<Item = T>
  where
    Self: 'b,
    'a: 'b,
    T: 'b;
  fn iter<'b>(&'b self) -> Self::Iter<'b>
  where
    'a: 'b;
  fn map(&self, f: impl FnMut(T) -> T) -> Self;
}

impl<'a, T: 'a + Copy> UnionLike<'a, T> for Vec<T> {
  fn len(&self) -> usize {
    self.len()
  }
  type Iter<'b>
    = Copied<slice::Iter<'b, T>>
  where
    Self: 'b,
    'a: 'b,
    T: 'b;
  fn iter<'b>(&'b self) -> Self::Iter<'b>
  where
    'a: 'b,
  {
    self.as_slice().iter().copied()
  }
  fn map(&self, f: impl FnMut(T) -> T) -> Self {
    self.iter().map(f).collect()
  }
}

impl<'a, T: 'a + Copy> UnionLike<'a, T> for (T, T) {
  fn len(&self) -> usize {
    2
  }
  type Iter<'b>
    = array::IntoIter<T, 2>
  where
    Self: 'b,
    'a: 'b,
    T: 'b;
  fn iter<'b>(&'b self) -> Self::Iter<'b>
  where
    'a: 'b,
  {
    [self.0, self.1].into_iter()
  }
  fn map(&self, mut f: impl FnMut(T) -> T) -> Self {
    (f(self.0), f(self.1))
  }
}
