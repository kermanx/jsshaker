use oxc::ast::ast::{IdentifierName, JSXIdentifier};

use crate::{analyzer::Analyzer, ast::AstKind2, entity::Entity, transformer::Transformer};

impl<'a> Analyzer<'a> {
  pub fn exec_jsx_identifier(&mut self, node: &'a JSXIdentifier<'a>) -> Entity<'a> {
    self.exec_mangable_static_string(AstKind2::JSXIdentifier(node), node.name.as_str())
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_jsx_identifier(&self, node: &'a JSXIdentifier<'a>) -> JSXIdentifier<'a> {
    let JSXIdentifier { span, name } = node;
    self.ast_builder.jsx_identifier(
      *span,
      self.transform_mangable_static_string(AstKind2::JSXIdentifier(node), name),
    )
  }

  pub fn transform_jsx_identifier_as_identifier_name(
    &self,
    node: &'a JSXIdentifier<'a>,
  ) -> IdentifierName<'a> {
    let JSXIdentifier { span, name } = node;
    self.ast_builder.identifier_name(
      *span,
      self.transform_mangable_static_string(AstKind2::JSXIdentifier(node), name),
    )
  }
}
