use oxc::ast::ast::{
  BigIntLiteral, BooleanLiteral, Expression, NullLiteral, NumberBase, NumericLiteral,
  RegExpLiteral, StringLiteral,
};

use crate::{Analyzer, entity::Entity, transformer::Transformer, utils::ast::AstKind2};

impl<'a> Analyzer<'a> {
  pub fn exec_string_literal(&mut self, node: &'a StringLiteral) -> Entity<'a> {
    self.exec_mangable_static_string(AstKind2::StringLiteral(node), &node.value)
  }

  pub fn exec_numeric_literal(&mut self, node: &'a NumericLiteral) -> Entity<'a> {
    if node.base == NumberBase::Float {
      self.factory.unknown_number
    } else {
      self.factory.number(node.value)
    }
  }

  pub fn exc_big_int_literal(&mut self, node: &'a BigIntLiteral) -> Entity<'a> {
    self.factory.big_int(&node.value)
  }

  pub fn exec_boolean_literal(&mut self, node: &'a BooleanLiteral) -> Entity<'a> {
    self.factory.boolean(node.value)
  }

  pub fn exec_null_literal(&mut self, _node: &'a NullLiteral) -> Entity<'a> {
    self.factory.null
  }

  pub fn exec_regexp_literal(&mut self, _node: &'a RegExpLiteral<'a>) -> Entity<'a> {
    self.factory.unknown
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_string_literal(
    &self,
    node: &'a StringLiteral,
    need_val: bool,
  ) -> Option<Expression<'a>> {
    let StringLiteral { span, value, .. } = node;
    need_val.then(|| {
      self.ast.expression_string_literal(
        *span,
        self.transform_mangable_static_string(AstKind2::StringLiteral(node), value),
        None,
      )
    })
  }
}
