use oxc::ast::ast::IdentifierName;

use crate::{analyzer::Analyzer, ast::AstKind2, entity::Entity, transformer::Transformer};

impl<'a> Analyzer<'a> {
  pub fn exec_identifier_name(&mut self, node: &'a IdentifierName<'a>) -> Entity<'a> {
    self.exec_mangable_static_string(AstKind2::IdentifierName(node), node.name.as_str())
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_identifier_name(&self, node: &'a IdentifierName<'a>) -> IdentifierName<'a> {
    let IdentifierName { span, name } = node;
    self.ast.identifier_name(
      *span,
      self.transform_mangable_static_string(AstKind2::IdentifierName(node), name),
    )
  }
}
