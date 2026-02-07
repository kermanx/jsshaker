use oxc::ast::ast::{Statement, ThrowStatement};

use crate::{analyzer::Analyzer, transformer::Transformer};

impl<'a> Analyzer<'a> {
  pub fn exec_throw_statement(&mut self, node: &'a ThrowStatement<'a>) {
    let value = self.exec_expression(&node.argument);
    self.exit_by_throw(true);
    self.include(value);
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_throw_statement(&self, node: &'a ThrowStatement<'a>) -> Option<Statement<'a>> {
    let ThrowStatement { span, argument } = node;
    let argument = self.transform_expression(argument, true).unwrap();
    Some(self.ast.statement_throw(*span, argument))
  }
}
