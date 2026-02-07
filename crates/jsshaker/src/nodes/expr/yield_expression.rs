use oxc::ast::ast::{Expression, YieldExpression};

use crate::{analyzer::Analyzer, entity::Entity, transformer::Transformer};

impl<'a> Analyzer<'a> {
  pub fn exec_yield_expression(&mut self, node: &'a YieldExpression<'a>) -> Entity<'a> {
    self.global_effect();

    if let Some(argument) = &node.argument {
      let argument = self.exec_expression(argument);
      argument.include(self);
    }
    self.factory.unknown
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_yield_expression(
    &self,
    node: &'a YieldExpression<'a>,
    _need_val: bool,
  ) -> Option<Expression<'a>> {
    let YieldExpression { span, delegate, argument } = node;

    let argument = argument.as_ref().map(|node| self.transform_expression(node, true).unwrap());

    Some(self.ast.expression_yield(*span, *delegate, argument))
  }
}
