use crate::{analyzer::Analyzer, define_box_bump_idx, dep::CustomDepTrait};

define_box_bump_idx! {
  pub struct MangleAtom;
}

impl<'a> CustomDepTrait<'a> for MangleAtom {
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    analyzer.mangler.mark_atom_non_mangable(*self);
  }
}
