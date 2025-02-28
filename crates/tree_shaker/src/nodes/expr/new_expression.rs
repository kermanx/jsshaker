use crate::{
  analyzer::Analyzer, ast::AstKind2, build_effect, entity::Entity, transformer::Transformer,
};
use oxc::ast::ast::{Expression, NewExpression, TSTypeParameterInstantiation};

impl<'a> Analyzer<'a> {
  pub fn exec_new_expression(&mut self, node: &'a NewExpression<'a>) -> Entity<'a> {
    let pure = self.has_pure_notation(node.span);

    self.scoping.pure += pure;
    let callee = self.exec_expression(&node.callee);
    self.scoping.pure -= pure;

    let arguments = self.exec_arguments(&node.arguments);

    self.scoping.pure += pure;
    let value = callee.construct(self, self.consumable(AstKind2::NewExpression(node)), arguments);
    self.scoping.pure -= pure;

    value
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_new_expression(
    &self,
    node: &'a NewExpression<'a>,
    need_val: bool,
  ) -> Option<Expression<'a>> {
    let NewExpression { span, callee, arguments, .. } = node;

    if need_val || self.is_referred(AstKind2::NewExpression(node)) {
      let callee = self.transform_expression(callee, true);
      let arguments = self.transform_arguments_need_call(arguments);

      Some(self.ast_builder.expression_new(
        *span,
        callee.unwrap(),
        arguments,
        None::<TSTypeParameterInstantiation>,
      ))
    } else {
      let callee = self.transform_expression(callee, false);
      let arguments = self.transform_arguments_no_call(arguments);
      build_effect!(self.ast_builder, *span, callee, arguments)
    }
  }
}
