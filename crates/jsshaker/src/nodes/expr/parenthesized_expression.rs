use oxc::ast::ast::{Expression, ParenthesizedExpression};

use crate::{analyzer::Analyzer, entity::Entity, transformer::Transformer};

impl<'a> Analyzer<'a> {
  pub fn exec_parenthesized_expression(
    &mut self,
    node: &'a ParenthesizedExpression<'a>,
  ) -> Entity<'a> {
    self.exec_expression(&node.expression)
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_parenthesized_expression(
    &self,
    node: &'a ParenthesizedExpression<'a>,
    need_val: bool,
  ) -> Option<Expression<'a>> {
    let ParenthesizedExpression { span, expression } = node;
    self
      .transform_expression(expression, need_val)
      .map(|expression| self.ast.expression_parenthesized(*span, expression))
  }
}
