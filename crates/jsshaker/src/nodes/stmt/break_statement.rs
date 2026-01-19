use oxc::ast::ast::{BreakStatement, Statement};

use crate::{analyzer::Analyzer, transformer::Transformer};

impl<'a> Analyzer<'a> {
  pub fn exec_break_statement(&mut self, node: &'a BreakStatement<'a>) {
    self.break_to_label(node.label.as_ref());
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_break_statement(&self, node: &'a BreakStatement<'a>) -> Option<Statement<'a>> {
    let BreakStatement { span, label } = node;

    Some(self.ast.statement_break(*span, label.clone()))
  }
}
