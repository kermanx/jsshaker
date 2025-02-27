use crate::{
  analyzer::Analyzer, consumable::ConsumableTrait, entity::Entity, mangling::MangleConstraint,
};

#[derive(Debug, Clone, Copy)]
pub struct ManglingDep<'a> {
  pub deps: (Entity<'a>, Entity<'a>),
  pub constraint: &'a MangleConstraint,
}

impl<'a> ConsumableTrait<'a> for ManglingDep<'a> {
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    self.deps.0.consume_mangable(analyzer);
    self.deps.1.consume_mangable(analyzer);
    analyzer.consume(self.constraint);
  }
}
