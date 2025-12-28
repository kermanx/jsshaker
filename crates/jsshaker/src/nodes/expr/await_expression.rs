use oxc::ast::ast::{AwaitExpression, Expression};

use crate::{analyzer::Analyzer, entity::Entity, transformer::Transformer, utils::ast::AstKind2};

impl<'a> Analyzer<'a> {
  pub fn exec_await_expression(&mut self, node: &'a AwaitExpression<'a>) -> Entity<'a> {
    let call_scope = self.call_scope_mut();
    if !call_scope.is_async {
      self.add_diagnostic("SyntaxError: await is only valid in async functions");
    }

    self.global_effect();

    let value = self.exec_expression(&node.argument);
    value.r#await(self, AstKind2::AwaitExpression(node))
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_await_expression(
    &self,
    node: &'a AwaitExpression<'a>,
    need_val: bool,
  ) -> Option<Expression<'a>> {
    let AwaitExpression { span, argument } = node;

    let has_effect = self.is_referred(AstKind2::AwaitExpression(node));

    if has_effect {
      let argument = self.transform_expression(argument, true).unwrap();
      Some(self.ast.expression_await(*span, argument))
    } else {
      self.transform_expression(argument, need_val)
    }
  }
}
