use oxc::ast::ast::PrivateIdentifier;

use crate::{analyzer::Analyzer, ast::AstKind2, entity::Entity, transformer::Transformer};

impl<'a> Analyzer<'a> {
  pub fn exec_private_identifier(&mut self, node: &'a PrivateIdentifier<'a>) -> Entity<'a> {
    // FIXME: Not good
    self.exec_mangable_static_string(
      AstKind2::PrivateIdentifier(node),
      self.escape_private_identifier_name(node.name.as_str()),
    )
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_private_identifier(
    &self,
    node: &'a PrivateIdentifier<'a>,
  ) -> PrivateIdentifier<'a> {
    let PrivateIdentifier { span, name } = node;
    self.ast_builder.private_identifier(
      *span,
      self.transform_mangable_static_string(AstKind2::PrivateIdentifier(node), name),
    )
  }
}
