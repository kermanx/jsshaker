use oxc::{
  allocator,
  ast::ast::{Expression, JSXSpreadChild},
};

use crate::{analyzer::Analyzer, entity::Entity, transformer::Transformer};

impl<'a> Analyzer<'a> {
  pub fn exec_jsx_spread_child(&mut self, _node: &'a JSXSpreadChild<'a>) -> Entity<'a> {
    self.factory.immutable_unknown
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_jsx_spread_child_effect_only(
    &self,
    node: &'a JSXSpreadChild<'a>,
  ) -> Option<Expression<'a>> {
    self.transform_expression(&node.expression, false)
  }

  pub fn transform_jsx_spread_child_need_val(
    &self,
    node: &'a JSXSpreadChild<'a>,
  ) -> allocator::Box<'a, JSXSpreadChild<'a>> {
    let JSXSpreadChild { span, expression } = node;

    self
      .ast_builder
      .alloc_jsx_spread_child(*span, self.transform_expression(expression, true).unwrap())
  }
}
