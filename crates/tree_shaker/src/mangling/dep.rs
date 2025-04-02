use crate::{analyzer::Analyzer, dep::CustomDepTrait, entity::Entity, mangling::MangleConstraint};

#[derive(Debug, Clone, Copy)]
pub struct ManglingDep<'a> {
  pub deps: (Entity<'a>, Entity<'a>),
  pub constraint: MangleConstraint<'a>,
}

impl<'a> CustomDepTrait<'a> for ManglingDep<'a> {
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    self.deps.0.consume_mangable(analyzer);
    self.deps.1.consume_mangable(analyzer);
    analyzer.consume(self.constraint);
  }
}

#[derive(Debug, Clone, Copy)]
pub struct AlwaysMangableDep<'a> {
  pub dep: Entity<'a>,
}

impl<'a> CustomDepTrait<'a> for AlwaysMangableDep<'a> {
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    self.dep.consume_mangable(analyzer);
  }
}
