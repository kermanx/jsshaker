use oxc::{
  ast::ast::{Statement, ThrowStatement},
  span::GetSpan,
};

use crate::{analyzer::Analyzer, ast::AstKind2, transformer::Transformer};

impl<'a> Analyzer<'a> {
  pub fn exec_throw_statement(&mut self, node: &'a ThrowStatement<'a>) {
    let value = self.exec_expression(&node.argument);

    let dep = AstKind2::ThrowStatement(node);

    self.explicit_throw(self.factory.computed(value, dep));
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_throw_statement(&self, node: &'a ThrowStatement<'a>) -> Option<Statement<'a>> {
    let need_val = self.is_referred(AstKind2::ThrowStatement(node));

    let ThrowStatement { span, argument } = node;

    let argument_span = argument.span();

    let argument = self
      .transform_expression(argument, need_val)
      .unwrap_or_else(|| self.build_unused_expression(argument_span));

    Some(self.ast_builder.statement_throw(*span, argument))
  }
}
