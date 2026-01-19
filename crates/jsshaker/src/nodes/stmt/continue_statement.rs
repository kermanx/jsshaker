use oxc::ast::ast::{ContinueStatement, Statement};

use crate::{analyzer::Analyzer, transformer::Transformer};

impl<'a> Analyzer<'a> {
  pub fn exec_continue_statement(&mut self, node: &'a ContinueStatement<'a>) {
    self.continue_to_label(node.label.as_ref());
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_continue_statement(
    &self,
    node: &'a ContinueStatement<'a>,
  ) -> Option<Statement<'a>> {
    let ContinueStatement { span, label } = node;

    Some(self.ast.statement_continue(*span, label.clone()))
  }
}
