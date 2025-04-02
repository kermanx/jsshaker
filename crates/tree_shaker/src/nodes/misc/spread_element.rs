use oxc::ast::ast::{ArrayExpressionElement, SpreadElement};

use crate::{analyzer::Analyzer, ast::AstKind2, entity::Entity, transformer::Transformer};

impl<'a> Analyzer<'a> {
  pub fn exec_spread_element(&mut self, node: &'a SpreadElement<'a>) -> Option<Entity<'a>> {
    let argument = self.exec_expression(&node.argument);
    argument.iterate_result_union(self, AstKind2::SpreadElement(node))
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_spread_element(
    &self,
    node: &'a SpreadElement<'a>,
    need_val: bool,
  ) -> Option<ArrayExpressionElement<'a>> {
    let SpreadElement { span, argument } = node;

    let need_spread = need_val || self.is_referred(AstKind2::SpreadElement(node));

    let argument = self.transform_expression(argument, need_spread);

    argument.map(|argument| {
      if need_spread {
        self.ast_builder.array_expression_element_spread_element(*span, argument)
      } else {
        argument.into()
      }
    })
  }
}
