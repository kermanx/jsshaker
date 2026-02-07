use crate::{analyzer::Analyzer, dep::CustomDepTrait, entity::Entity, mangling::MangleConstraint};

#[derive(Debug, Clone, Copy)]
pub struct ManglingDep<'a> {
  pub deps: (Entity<'a>, Entity<'a>),
  pub constraint: MangleConstraint<'a>,
}

impl<'a> CustomDepTrait<'a> for ManglingDep<'a> {
  fn include(&self, analyzer: &mut Analyzer<'a>) {
    self.deps.0.include_mangable(analyzer);
    self.deps.1.include_mangable(analyzer);
    analyzer.include(self.constraint);
  }
}

#[derive(Debug, Clone, Copy)]
pub struct AlwaysMangableDep<'a> {
  pub dep: Entity<'a>,
}

impl<'a> CustomDepTrait<'a> for AlwaysMangableDep<'a> {
  fn include(&self, analyzer: &mut Analyzer<'a>) {
    self.dep.include_mangable(analyzer);
  }
}
