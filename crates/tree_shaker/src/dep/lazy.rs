use std::cell::RefCell;

use oxc::allocator;

use super::{CustomDepTrait, DepTrait};
use crate::analyzer::Analyzer;

#[derive(Debug, Clone, Copy)]
pub struct LazyDep<'a, T: DepTrait<'a> + 'a>(pub &'a RefCell<Option<allocator::Vec<'a, T>>>);

impl<'a, T: DepTrait<'a> + 'a> CustomDepTrait<'a> for LazyDep<'a, T> {
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    analyzer.consume(self.0.take());
  }
}

impl<'a, T: DepTrait<'a> + 'a> LazyDep<'a, T> {
  pub fn push(&self, analyzer: &mut Analyzer<'a>, dep: T) {
    let mut deps_ref = self.0.borrow_mut();
    if let Some(deps) = deps_ref.as_mut() {
      deps.push(dep);
    } else {
      drop(deps_ref);
      analyzer.consume(dep);
    }
  }
}
