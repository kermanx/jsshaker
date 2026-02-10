use rustc_hash::FxHashSet;

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
  fn include(&self, analyzer: &mut Analyzer<'a>) {
    let data = analyzer.folder.bump.get_mut(self.data);
    match data {
      FoldingData::Initial => {
        if matches!(self.literal, LiteralValue::String(s,_) if s.len() > analyzer.config.max_folding_string_length)
        {
          let same_folding = analyzer
            .folder
            .long_strings
            .entry(self.literal)
            .or_insert_with(|| Some(FxHashSet::default()));
          if let Some(set) = same_folding {
            if set.len() > 1 {
              for data_id in same_folding.take().unwrap() {
                analyzer.mark_non_foldable(data_id);
              }
              analyzer.mark_non_foldable(self.data);
              self.value.include_mangable(analyzer);
              return;
            }
            set.insert(self.data);
          } else {
            analyzer.mark_non_foldable(self.data);
            self.value.include_mangable(analyzer);
            return;
          }
        }

        *data = FoldingData::Foldable {
          literal: self.literal,
          used_values: analyzer.factory.vec1(self.value),
          mangle_atom: self.mangle_atom,
        };
      }
      FoldingData::Foldable { literal, used_values, mangle_atom, .. } => {
        if !literal.strict_eq(self.literal, true).0 {
          analyzer.mark_non_foldable(self.data);
          self.value.include_mangable(analyzer);
          return;
        }

        used_values.push(self.value);
        match (*mangle_atom, self.mangle_atom) {
          (Some(m1), Some(m2)) => {
            analyzer.include(MangleConstraint::Eq(m1, m2));
          }
          (None, Some(m)) | (Some(m), None) => {
            analyzer.include(m);
          }
          _ => {}
        }
      }
      FoldingData::NonFoldable => {
        self.value.include_mangable(analyzer);
      }
    }
  }
}

#[derive(Debug, Clone, Copy)]
pub struct NonFoldableDep {
  pub data: FoldingDataId,
}

impl<'a> CustomDepTrait<'a> for NonFoldableDep {
  fn include(&self, analyzer: &mut Analyzer<'a>) {
    analyzer.mark_non_foldable(self.data);
  }
}
