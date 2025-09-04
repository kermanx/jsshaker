use oxc_index::define_index_type;

use crate::{analyzer::Analyzer, dep::CustomDepTrait};

define_index_type! {
  pub struct MangleAtom = u32;
  DISABLE_MAX_INDEX_CHECK = cfg!(not(debug_assertions));
}

impl<'a> CustomDepTrait<'a> for MangleAtom {
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    analyzer.mangler.mark_atom_non_mangable(*self);
  }
}
