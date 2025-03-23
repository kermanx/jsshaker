use std::{cell::RefCell, mem};

use crate::{
  analyzer::Analyzer,
  dep::CustomDepTrait,
  entity::{Entity, LiteralEntity},
  mangling::MangleAtom,
};

use super::{FoldingData, FoldingState};

#[derive(Debug)]
pub struct FoldableDep<'a> {
  pub data: &'a RefCell<FoldingData<'a>>,
  pub literal: LiteralEntity<'a>,
  pub value: Entity<'a>,
  pub mangle_atom: Option<MangleAtom>,
}

impl<'a> CustomDepTrait<'a> for FoldableDep<'a> {
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    let mut data = self.data.borrow_mut();

    if data.state.is_foldable() {
      data.used_values.push(self.value);
      if let Some(mangle_atom) = self.mangle_atom {
        data.used_mangle_atoms.push(mangle_atom);
      }
    }

    match data.state {
      FoldingState::Initial => {
        data.state = FoldingState::Foldable(self.literal);
      }
      FoldingState::Foldable(literal) => {
        if literal != self.literal {
          data.state = FoldingState::UnFoldable;
        }
      }
      FoldingState::UnFoldable => {
        mem::drop(data);
        self.value.consume(analyzer);
      }
    }
  }
}

#[derive(Debug, Clone, Copy)]
pub struct UnFoldableDep<'a> {
  pub data: &'a RefCell<FoldingData<'a>>,
}

impl<'a> CustomDepTrait<'a> for UnFoldableDep<'a> {
  fn consume(&self, _analyzer: &mut Analyzer<'a>) {
    let mut data = self.data.borrow_mut();
    data.state = FoldingState::UnFoldable;
  }
}
