use oxc::ast::ast::ForStatementLeft;

use crate::{analyzer::Analyzer, entity::Entity, transformer::Transformer};

impl<'a> Analyzer<'a> {
  pub fn declare_for_statement_left(&mut self, node: &'a ForStatementLeft<'a>) {
    if let ForStatementLeft::VariableDeclaration(node) = node {
      self.declare_variable_declaration(node, None);
    }
  }

  pub fn init_for_statement_left(&mut self, node: &'a ForStatementLeft<'a>, init: Entity<'a>) {
    match node {
      ForStatementLeft::VariableDeclaration(node) => {
        self.init_variable_declaration(node, Some(init));
      }
      _ => self.exec_assignment_target_write(node.to_assignment_target(), init, None),
    }
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_for_statement_left(
    &self,
    node: &'a ForStatementLeft<'a>,
  ) -> Option<ForStatementLeft<'a>> {
    match node {
      ForStatementLeft::VariableDeclaration(node) => {
        self.transform_variable_declaration(node).map(ForStatementLeft::VariableDeclaration)
      }
      _ => self
        .transform_assignment_target_write(node.to_assignment_target(), false, false)
        .1
        .map(ForStatementLeft::from),
    }
  }
}
