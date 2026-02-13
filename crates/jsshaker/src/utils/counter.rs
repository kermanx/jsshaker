use std::{collections::hash_map::Entry, hash::Hash};

use rustc_hash::{FxHashMap, FxHashSet};

#[derive(Debug, Clone)]
pub struct Counter<T: Eq + Copy + Hash>(FxHashMap<T, usize>);

impl<T: Eq + Copy + Hash> Default for Counter<T> {
  fn default() -> Self {
    Counter(FxHashMap::default())
  }
}

impl<T: Eq + Copy + Hash> Counter<T> {
  pub fn increment(&mut self, key: T) {
    *self.0.entry(key).or_default() += 1;
  }

  pub fn decrement(&mut self, key: T) {
    if let Entry::Occupied(mut entry) = self.0.entry(key) {
      let count = entry.get_mut();
      *count -= 1;
      if *count == 0 {
        entry.remove();
      }
    }
  }

  pub fn get(&self, key: T) -> usize {
    *self.0.get(&key).unwrap_or(&0)
  }

  pub fn has(&self, key: T) -> bool {
    self.0.contains_key(&key)
  }

  pub fn extend(&mut self, other: &FxHashSet<T>) {
    for key in other {
      self.increment(*key);
    }
  }

  pub fn unextend(&mut self, other: &FxHashSet<T>) {
    for key in other {
      self.decrement(*key);
    }
  }
}
