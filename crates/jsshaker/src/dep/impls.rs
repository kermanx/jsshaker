use oxc::allocator;

use super::{CustomDepTrait, DepTrait};
use crate::analyzer::Analyzer;

impl<'a> CustomDepTrait<'a> for () {
  fn include(&self, _: &mut Analyzer<'a>) {}
}

impl<'a, T: DepTrait<'a> + 'a> CustomDepTrait<'a> for allocator::Box<'a, T> {
  fn include(&self, analyzer: &mut Analyzer<'a>) {
    self.as_ref().include(analyzer)
  }
}

impl<'a, T: DepTrait<'a> + 'a> CustomDepTrait<'a> for Option<T> {
  fn include(&self, analyzer: &mut Analyzer<'a>) {
    if let Some(value) = self {
      value.include(analyzer)
    }
  }
}

impl<'a, T: DepTrait<'a> + 'a> CustomDepTrait<'a> for &'a [T] {
  fn include(&self, analyzer: &mut Analyzer<'a>) {
    for item in *self {
      item.include(analyzer)
    }
  }
}

impl<'a, T: DepTrait<'a> + 'a> CustomDepTrait<'a> for Vec<T> {
  fn include(&self, analyzer: &mut Analyzer<'a>) {
    for item in self {
      item.include(analyzer)
    }
  }
}

impl<'a, T: DepTrait<'a> + 'a> CustomDepTrait<'a> for allocator::Vec<'a, T> {
  fn include(&self, analyzer: &mut Analyzer<'a>) {
    for item in self {
      item.include(analyzer)
    }
  }
}

impl<'a, T: DepTrait<'a> + 'a> CustomDepTrait<'a> for &allocator::Vec<'a, T> {
  fn include(&self, analyzer: &mut Analyzer<'a>) {
    for item in *self {
      item.include(analyzer)
    }
  }
}

impl<'a, T1: DepTrait<'a> + 'a, T2: DepTrait<'a> + 'a> CustomDepTrait<'a> for (T1, T2) {
  fn include(&self, analyzer: &mut Analyzer<'a>) {
    self.0.include(analyzer);
    self.1.include(analyzer)
  }
}

impl<'a, T1: DepTrait<'a> + 'a, T2: DepTrait<'a> + 'a, T3: DepTrait<'a> + 'a> CustomDepTrait<'a>
  for (T1, T2, T3)
{
  fn include(&self, analyzer: &mut Analyzer<'a>) {
    self.0.include(analyzer);
    self.1.include(analyzer);
    self.2.include(analyzer);
  }
}

impl<'a, T1: DepTrait<'a> + 'a, T2: DepTrait<'a> + 'a, T3: DepTrait<'a> + 'a, T4: DepTrait<'a> + 'a>
  CustomDepTrait<'a> for (T1, T2, T3, T4)
{
  fn include(&self, analyzer: &mut Analyzer<'a>) {
    self.0.include(analyzer);
    self.1.include(analyzer);
    self.2.include(analyzer);
    self.3.include(analyzer);
  }
}
