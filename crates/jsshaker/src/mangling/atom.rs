use std::ptr::NonNull;

use crate::{analyzer::Analyzer, define_box_bump_idx, dep::CustomDepTrait, mangling::AtomState};

define_box_bump_idx! {
  pub struct MangleAtom;
}

impl<'a> CustomDepTrait<'a> for MangleAtom {
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    analyzer.mangler.mark_atom_non_mangable(*self);
  }
}

pub const BUILTIN_ATOM: MangleAtom =
  MangleAtom { ptr: NonNull::from_ref(&AtomState::Preserved).cast() };
