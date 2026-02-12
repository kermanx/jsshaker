use std::cell::Cell;

#[derive(Debug, PartialEq, Eq)]
pub struct Version(Cell<usize>);

impl Default for Version {
  fn default() -> Self {
    Version(Cell::new(1))
  }
}

impl Version {
  pub fn trackable(&self) -> bool {
    self.0.get() != 0
  }

  pub fn untrack(&self) {
    self.0.set(0);
  }

  pub fn increment(&self) -> bool {
    if !self.trackable() {
      return false;
    }
    self.0.set(self.0.get() + 1);
    true
  }

  pub fn get(&self) -> usize {
    self.0.get()
  }
}
