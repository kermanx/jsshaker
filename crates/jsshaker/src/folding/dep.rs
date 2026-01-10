use super::FoldingData;
use crate::{
  analyzer::Analyzer,
  dep::CustomDepTrait,
  entity::Entity,
  folding::FoldingDataId,
  mangling::{MangleAtom, MangleConstraint},
  value::LiteralValue,
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
    match data {
      FoldingData::Initial => {
        *data = FoldingData::Foldable {
          literal: self.literal,
          used_values: analyzer.factory.vec1(self.value),
          mangle_atom: self.mangle_atom,
        };
      }
      FoldingData::Foldable { literal, used_values, mangle_atom, .. } => {
        if literal.strict_eq(self.literal, true).0 {
          used_values.push(self.value);
          match (*mangle_atom, self.mangle_atom) {
            (Some(m1), Some(m2)) => {
              analyzer.consume(MangleConstraint::Eq(m1, m2));
            }
            (None, Some(m)) | (Some(m), None) => {
              analyzer.consume(m);
            }
            _ => {}
          }
        } else {
          analyzer.mark_unfoldable(self.data);
          self.value.consume_mangable(analyzer);
        }
      }
      FoldingData::UnFoldable => {
        self.value.consume_mangable(analyzer);
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
    analyzer.mark_unfoldable(self.data);
  }
}
