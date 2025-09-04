use oxc::ast::ast::{BinaryExpression, Expression};

use crate::{analyzer::Analyzer, build_effect, entity::Entity, transformer::Transformer};

impl<'a> Analyzer<'a> {
  pub fn exec_binary_expression(&mut self, node: &'a BinaryExpression<'a>) -> Entity<'a> {
    let lhs = self.exec_expression(&node.left);
    let rhs = self.exec_expression(&node.right);

    self.op_binary(node.operator, lhs, rhs)
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_binary_expression(
    &self,
    node: &'a BinaryExpression<'a>,
    need_val: bool,
  ) -> Option<Expression<'a>> {
    let BinaryExpression { span, operator, left, right } = node;

    let left = self.transform_expression(left, need_val);
    let right = self.transform_expression(right, need_val);

    if need_val {
      Some(self.ast_builder.expression_binary(*span, left.unwrap(), *operator, right.unwrap()))
    } else {
      build_effect!(self.ast_builder, *span, left, right)
    }
  }
}
