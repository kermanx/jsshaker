mod dep;

use std::mem;

use dep::{FoldableDep, UnFoldableDep};
use oxc::{allocator, ast::ast::Expression, span::GetSpan};
use rustc_hash::FxHashMap;

use crate::{
  analyzer::Analyzer,
  define_box_bump_idx,
  dep::DepAtom,
  entity::Entity,
  mangling::MangleAtom,
  transformer::Transformer,
  utils::{ast::AstKind2, box_bump::BoxBump},
  value::LiteralValue,
};

#[derive(Debug)]
pub enum FoldingData<'a> {
  Initial,
  Foldable {
    literal: &'a LiteralValue<'a>,
    used_values: allocator::Vec<'a, Entity<'a>>,
    mangle_atom: Option<MangleAtom>,
  },
  UnFoldable,
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
    let data = *self
      .folder
      .nodes
      .entry(node.into())
      .or_insert_with(|| self.folder.bump.alloc(FoldingData::Initial));
    if let FoldingData::UnFoldable = self.folder.bump.get(data) {
      value
    } else if let Some(mut literal) = self.get_foldable_literal(value) {
      let mangle_atom = match &mut literal {
        LiteralValue::String(_, atom) => Some(*atom.get_or_insert_with(|| self.mangler.new_atom())),
        _ => None,
      };
      let literal = &*self.factory.alloc(literal);
      self.factory.computed(literal.into(), FoldableDep { data, literal, value, mangle_atom })
    } else {
      self.factory.computed(value, UnFoldableDep { data })
    }
  }

  pub fn mark_unfoldable(&mut self, id: FoldingDataId) {
    let data = self.folder.bump.get_mut(id);
    match mem::replace(data, FoldingData::UnFoldable) {
      FoldingData::UnFoldable | FoldingData::Initial => {}
      FoldingData::Foldable { used_values, .. } => {
        for value in used_values {
          value.consume_mangable(self);
        }
      }
    }
  }
}

impl<'a> Transformer<'a> {
  pub fn build_folded_expr(&self, node: AstKind2) -> Option<Expression<'a>> {
    let data = self.folder.get(node.into())?;
    if let FoldingData::Foldable { literal, mangle_atom, .. } = data {
      Some(literal.build_expr(self, node.span(), *mangle_atom))
    } else {
      None
    }
  }

  pub fn get_folded_literal(&self, node: AstKind2<'a>) -> Option<LiteralValue<'a>> {
    let data = self.folder.get(node.into())?;
    if let FoldingData::Foldable { literal, .. } = data { Some(**literal) } else { None }
  }
}
