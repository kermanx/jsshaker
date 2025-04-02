use oxc::{
  ast::ast::{Expression, SequenceExpression},
  span::SPAN,
};

use crate::{analyzer::Analyzer, entity::Entity, transformer::Transformer};

impl<'a> Analyzer<'a> {
  pub fn exec_sequence_expression(&mut self, node: &'a SequenceExpression<'a>) -> Entity<'a> {
    let mut last = None;
    for expression in &node.expressions {
      last = Some(self.exec_expression(expression));
    }
    last.unwrap()
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_sequence_expression(
    &self,
    node: &'a SequenceExpression<'a>,
    need_val: bool,
  ) -> Option<Expression<'a>> {
    let SequenceExpression { span, expressions } = node;

    let length = expressions.len();

    let mut transformed_expressions = self.ast_builder.vec();
    for (index, expression) in expressions.into_iter().enumerate() {
      if let Some(expr) = self.transform_expression(expression, need_val && index == length - 1) {
        transformed_expressions.push(expr);
      }
    }

    if transformed_expressions.is_empty() {
      None
    } else if transformed_expressions.len() == 1 {
      if self.config.preserve_function_name
        && matches!(transformed_expressions.last().unwrap(), Expression::FunctionExpression(_))
      {
        transformed_expressions.insert(0, self.build_unused_expression(SPAN));
        Some(self.ast_builder.expression_sequence(*span, transformed_expressions))
      } else {
        Some(transformed_expressions.pop().unwrap())
      }
    } else {
      Some(self.ast_builder.expression_sequence(*span, transformed_expressions))
    }
  }
}
