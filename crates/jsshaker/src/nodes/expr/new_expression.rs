use oxc::ast::{
  NONE,
  ast::{Expression, NewExpression},
};

use crate::{
  analyzer::Analyzer, ast::AstKind2, build_effect, entity::Entity, transformer::Transformer,
};

impl<'a> Analyzer<'a> {
  pub fn exec_new_expression(&mut self, node: &'a NewExpression<'a>) -> Entity<'a> {
    let callee = self.exec_expression(&node.callee);
    let arguments = self.exec_arguments(&node.arguments);
    let callsite = AstKind2::NewExpression(node);
    self.scoping.current_callsite = callsite;
    let result = callee.construct(self, callsite, arguments);
    self.scoping.current_callsite = AstKind2::ENVIRONMENT;
    result
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_new_expression(
    &self,
    node: &'a NewExpression<'a>,
    need_val: bool,
  ) -> Option<Expression<'a>> {
    let NewExpression { span, callee, arguments, .. } = node;

    if need_val || self.is_deoptimized(AstKind2::NewExpression(node)) {
      let callee = self.transform_expression(callee, true);
      let arguments = self.transform_arguments_need_call(arguments);

      Some(self.ast.expression_new(*span, callee.unwrap(), NONE, arguments))
    } else {
      let callee = self.transform_expression(callee, false);
      let arguments = self.transform_arguments_no_call(arguments);
      build_effect!(self.ast, *span, callee, arguments)
    }
  }
}
