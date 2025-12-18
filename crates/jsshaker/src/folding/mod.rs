mod dep;

use dep::{FoldableDep, UnFoldableDep};
use oxc::{allocator, ast::ast::Expression, span::GetSpan};
use rustc_hash::FxHashMap;

use crate::{
  analyzer::Analyzer,
  define_box_bump_idx,
  dep::DepAtom,
  entity::Entity,
  mangling::{MangleAtom, MangleConstraint},
  transformer::Transformer,
  utils::{ast::AstKind2, box_bump::BoxBump},
  value::LiteralValue,
};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum FoldingState<'a> {
  #[default]
  Initial,
  Foldable(LiteralValue<'a>),
  UnFoldable,
}

impl<'a> FoldingState<'a> {
  pub fn is_foldable(&self) -> bool {
    matches!(self, Self::Initial | Self::Foldable(_))
  }

  pub fn get_foldable_literal(self) -> Option<LiteralValue<'a>> {
    match self {
      Self::Initial => None, // Change to `unreachable!()` later
      Self::Foldable(literal) => Some(literal),
      Self::UnFoldable => None,
    }
  }
}

#[derive(Debug)]
pub struct FoldingData<'a> {
  pub state: FoldingState<'a>,
  pub used_values: allocator::Vec<'a, Entity<'a>>,
  pub used_mangle_atoms: allocator::Vec<'a, MangleAtom>,
  pub allocated_atom: Option<MangleAtom>,
}

define_box_bump_idx! {
  pub struct FoldingDataId;
}

pub struct ConstantFolder<'a> {
  bump: BoxBump<'a, FoldingDataId, FoldingData<'a>>,
  nodes: FxHashMap<DepAtom, FoldingDataId>,
}

impl<'a> ConstantFolder<'a> {
  pub fn new(allocator: &'a allocator::Allocator) -> Self {
    Self { bump: BoxBump::new(allocator), nodes: FxHashMap::default() }
  }

  pub fn get(&self, atom: DepAtom) -> Option<&FoldingData<'a>> {
    let data_id = self.nodes.get(&atom)?;
    Some(self.bump.get(*data_id))
  }
}

impl<'a> Analyzer<'a> {
  fn get_foldable_literal(&mut self, value: Entity<'a>) -> Option<LiteralValue<'a>> {
    if let Some(lit) = value.get_literal(self) {
      lit.can_build_expr(self).then_some(lit)
    } else {
      None
    }
  }

  pub fn try_fold_node(&mut self, node: AstKind2<'a>, value: Entity<'a>) -> Entity<'a> {
    let data = *self.folder.nodes.entry(node.into()).or_insert_with(|| {
      self.folder.bump.alloc(FoldingData {
        state: FoldingState::Initial,
        used_values: self.factory.vec(),
        used_mangle_atoms: self.factory.vec(),
        allocated_atom: None,
      })
    });
    if !self.folder.bump.get(data).state.is_foldable() {
      value
    } else if let Some(literal) = self.get_foldable_literal(value) {
      let (literal_value, mangle_atom) = match literal {
        LiteralValue::String(value, atom) => {
          let atom = atom.unwrap_or_else(|| {
            let data = self.folder.bump.get_mut(data);
            *data.allocated_atom.get_or_insert_with(|| self.mangler.new_atom())
          });
          (self.factory.alloc(LiteralValue::String(value, Some(atom))).into(), Some(atom))
        }
        _ => (self.factory.alloc(literal).into(), None),
      };
      self.factory.computed(literal_value, FoldableDep { data, literal, value, mangle_atom })
    } else {
      self.factory.computed(value, UnFoldableDep { data })
    }
  }

  pub fn post_analyze_handle_folding(&mut self) -> bool {
    let mut changed = false;
    for data in self.folder.nodes.values().copied().collect::<Vec<_>>() {
      let data = self.folder.bump.get_mut(data);
      if data.state.is_foldable() {
        if data.used_mangle_atoms.len() > 1 {
          let first_atom = data.used_mangle_atoms[0];
          for atom in data.used_mangle_atoms.drain(1..) {
            MangleConstraint::Eq(first_atom, atom).add_to_mangler(&mut self.mangler);
          }
        }
      } else {
        let values = data.used_values.drain(..).collect::<Vec<_>>();
        let allocated_atom = data.allocated_atom.take();
        for value in values {
          value.consume_mangable(self);
          changed = true;
        }
        self.consume(allocated_atom);
      }
    }
    changed
  }
}

impl<'a> Transformer<'a> {
  pub fn build_folded_expr(&self, node: AstKind2) -> Option<Expression<'a>> {
    let data = self.folder.get(node.into())?;
    data.state.get_foldable_literal().map(|literal| {
      let span = node.span();
      let mangle_atom = data.used_mangle_atoms.first().copied();
      literal.build_expr(self, span, mangle_atom)
    })
  }

  pub fn get_folded_literal(&self, node: AstKind2<'a>) -> Option<LiteralValue<'a>> {
    self.folder.get(node.into())?.state.get_foldable_literal()
  }
}
