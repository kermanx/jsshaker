mod dep;

use std::mem;

use dep::{FoldableDep, NonFoldableDep};
use oxc::{
  allocator,
  ast::{ast::Expression, match_member_expression},
  span::GetSpan,
};
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
    literal: LiteralValue<'a>,
    used_values: allocator::Vec<'a, Entity<'a>>,
    mangle_atom: Option<MangleAtom>,
  },
  NonFoldable,
}

define_box_bump_idx! {
  pub struct FoldingDataId for FoldingData<'static>;
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
    if let FoldingData::NonFoldable = self.folder.bump.get(data) {
      value
    } else if let Some(literal) = self.get_foldable_literal(value) {
      let mangle_atom = match literal {
        LiteralValue::String(_, Some(atom)) => Some(atom),
        _ => None,
      };
      value.override_dep(self.factory.dep(FoldableDep { data, literal, value, mangle_atom }))
    } else {
      self.factory.computed(value, NonFoldableDep { data })
    }
  }

  pub fn mark_non_foldable(&mut self, id: FoldingDataId) {
    let data = self.folder.bump.get_mut(id);
    match mem::replace(data, FoldingData::NonFoldable) {
      FoldingData::NonFoldable | FoldingData::Initial => {}
      FoldingData::Foldable { used_values, .. } => {
        for value in used_values {
          value.include_mangable(self);
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
    if let FoldingData::Foldable { literal, .. } = data { Some(*literal) } else { None }
  }
}

pub fn maybe_foldable_expr(node: &Expression) -> bool {
  match node {
    match_member_expression!(Expression) => true,

    Expression::StringLiteral(_)
    | Expression::NumericLiteral(_)
    | Expression::BigIntLiteral(_)
    | Expression::BooleanLiteral(_)
    | Expression::NullLiteral(_)
    | Expression::RegExpLiteral(_)
    | Expression::TemplateLiteral(_) => false,

    Expression::FunctionExpression(_)
    | Expression::ArrowFunctionExpression(_)
    | Expression::ObjectExpression(_)
    | Expression::ParenthesizedExpression(_)
    | Expression::SequenceExpression(_)
    | Expression::ImportExpression(_)
    | Expression::NewExpression(_)
    | Expression::ClassExpression(_) => false,

    Expression::Identifier(_)
    | Expression::UnaryExpression(_)
    | Expression::UpdateExpression(_)
    | Expression::BinaryExpression(_)
    | Expression::LogicalExpression(_)
    | Expression::ConditionalExpression(_)
    | Expression::CallExpression(_)
    | Expression::TaggedTemplateExpression(_)
    | Expression::AwaitExpression(_)
    | Expression::YieldExpression(_)
    | Expression::ArrayExpression(_)
    | Expression::AssignmentExpression(_)
    | Expression::ChainExpression(_)
    | Expression::MetaProperty(_)
    | Expression::PrivateInExpression(_)
    | Expression::ThisExpression(_)
    | Expression::Super(_) => true,

    Expression::JSXElement(_) | Expression::JSXFragment(_) => true,

    Expression::V8IntrinsicExpression(_)
    | Expression::TSAsExpression(_)
    | Expression::TSInstantiationExpression(_)
    | Expression::TSTypeAssertion(_)
    | Expression::TSNonNullExpression(_)
    | Expression::TSSatisfiesExpression(_) => unreachable!(),
  }
}
