use oxc::ast::ast::{ContinueStatement, Statement};

use crate::{analyzer::Analyzer, ast::AstKind2, transformer::Transformer};

impl<'a> Analyzer<'a> {
  pub fn exec_continue_statement(&mut self, node: &'a ContinueStatement<'a>) {
    let label = node.label.as_ref().map(|label| &label.name);
    if self.continue_to_label(label) {
      self.consume(AstKind2::ContinueStatement(node));
    }
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_continue_statement(
    &self,
    node: &'a ContinueStatement<'a>,
  ) -> Option<Statement<'a>> {
    let ContinueStatement { span, label } = node;

    Some(self.ast.statement_continue(
      *span,
      if self.is_included(AstKind2::ContinueStatement(node)) { label.clone() } else { None },
    ))
  }
}
