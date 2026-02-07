use std::cell::RefCell;

use oxc::allocator;

use super::{CustomDepTrait, DepTrait};
use crate::analyzer::Analyzer;

#[derive(Debug, Clone, Copy)]
pub struct LazyDep<'a, T: DepTrait<'a> + 'a>(pub &'a RefCell<Option<allocator::Vec<'a, T>>>);

impl<'a, T: DepTrait<'a> + 'a> CustomDepTrait<'a> for LazyDep<'a, T> {
  fn include(&self, analyzer: &mut Analyzer<'a>) {
    analyzer.include(self.0.take());
  }
}

impl<'a, T: DepTrait<'a> + 'a> LazyDep<'a, T> {
  pub fn push(&self, analyzer: &mut Analyzer<'a>, dep: T) {
    let mut deps_ref = self.0.borrow_mut();
    if let Some(deps) = deps_ref.as_mut() {
      deps.push(dep);
    } else {
      drop(deps_ref);
      analyzer.include(dep);
    }
  }

  pub fn push_defer(&self, dep: T, deferred_deps: &mut Vec<T>) {
    let mut deps_ref = self.0.borrow_mut();
    if let Some(deps) = deps_ref.as_mut() {
      deps.push(dep);
    } else {
      drop(deps_ref);
      deferred_deps.push(dep);
    }
  }

  pub fn is_included(&self) -> bool {
    self.0.borrow().is_none()
  }
}
