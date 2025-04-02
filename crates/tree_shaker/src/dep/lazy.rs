use std::cell::RefCell;

use super::{CustomDepTrait, Dep, DepVec};
use crate::analyzer::Analyzer;

#[derive(Debug, Clone, Copy)]
pub struct LazyDep<'a>(pub &'a RefCell<Option<DepVec<'a>>>);

impl<'a> CustomDepTrait<'a> for LazyDep<'a> {
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    self.0.take().consume(analyzer);
  }
}

impl<'a> LazyDep<'a> {
  pub fn push(&self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) {
    let mut deps_ref = self.0.borrow_mut();
    if let Some(deps) = deps_ref.as_mut() {
      deps.push(dep);
    } else {
      drop(deps_ref);
      analyzer.consume(dep);
    }
  }
}
