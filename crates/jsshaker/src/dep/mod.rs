mod assoc;
mod atom;
mod collector;
mod impls;
mod lazy;
mod once;

use std::fmt::Debug;

pub use atom::*;
pub use collector::*;
pub use lazy::*;
pub use once::*;
use oxc::allocator::{self, Allocator};

use crate::analyzer::Analyzer;

pub trait CustomDepTrait<'a>: Debug {
  fn consume(&self, analyzer: &mut Analyzer<'a>);
}

pub trait DepTrait<'a>: Debug {
  fn consume(&self, analyzer: &mut Analyzer<'a>);
  fn uniform(self, allocator: &'a Allocator) -> Dep<'a>;
}

impl<'a, T: CustomDepTrait<'a> + 'a> DepTrait<'a> for T {
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    self.consume(analyzer);
  }
  fn uniform(self, allocator: &'a Allocator) -> Dep<'a> {
    Dep(allocator.alloc(self))
  }
}

#[derive(Debug, Clone, Copy)]
pub struct Dep<'a>(pub &'a (dyn CustomDepTrait<'a> + 'a));

impl<'a> DepTrait<'a> for Dep<'a> {
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    self.0.consume(analyzer);
  }
  fn uniform(self, _: &'a Allocator) -> Dep<'a> {
    self
  }
}

pub type DepVec<'a> = allocator::Vec<'a, Dep<'a>>;

impl<'a> Analyzer<'a> {
  #[inline]
  pub fn consume(&mut self, dep: impl DepTrait<'a> + 'a) {
    dep.consume(self);
  }

  pub fn dep(&self, dep: impl CustomDepTrait<'a> + 'a) -> Dep<'a> {
    self.factory.dep(dep)
  }
}
