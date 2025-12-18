use super::FoldingState;
use crate::{
  analyzer::Analyzer, dep::CustomDepTrait, entity::Entity, folding::FoldingDataId,
  mangling::MangleAtom, value::LiteralValue,
};

#[derive(Debug)]
pub struct FoldableDep<'a> {
  pub data: FoldingDataId,
  pub literal: LiteralValue<'a>,
  pub value: Entity<'a>,
  pub mangle_atom: Option<MangleAtom>,
}

impl<'a> CustomDepTrait<'a> for FoldableDep<'a> {
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    let data = analyzer.folder.bump.get_mut(self.data);

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
        analyzer.consume(self.value);
      }
    }
  }
}

#[derive(Debug, Clone, Copy)]
pub struct UnFoldableDep {
  pub data: FoldingDataId,
}

impl<'a> CustomDepTrait<'a> for UnFoldableDep {
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    let data = analyzer.folder.bump.get_mut(self.data);
    data.state = FoldingState::UnFoldable;
  }
}
