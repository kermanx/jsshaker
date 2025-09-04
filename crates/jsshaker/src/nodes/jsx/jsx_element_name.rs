use oxc::ast::ast::{Expression, JSXElementName};

use crate::{analyzer::Analyzer, entity::Entity, transformer::Transformer};

impl<'a> Analyzer<'a> {
  pub fn exec_jsx_element_name(&mut self, node: &'a JSXElementName<'a>) -> Entity<'a> {
    match node {
      JSXElementName::Identifier(_node) => self.factory.unknown_string,
      JSXElementName::IdentifierReference(node) => self.exec_identifier_reference_read(node),
      JSXElementName::NamespacedName(_node) => self.factory.unknown_string,
      JSXElementName::MemberExpression(node) => self.exec_jsx_member_expression(node),
      JSXElementName::ThisExpression(node) => self.exec_this_expression(node),
    }
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_jsx_element_name_effect_only(
    &self,
    node: &'a JSXElementName<'a>,
  ) -> Option<Expression<'a>> {
    match node {
      JSXElementName::Identifier(_node) => None,
      JSXElementName::IdentifierReference(node) => {
        self.transform_identifier_reference_read(node, false).map(Expression::Identifier)
      }
      JSXElementName::NamespacedName(_node) => None,
      JSXElementName::MemberExpression(node) => {
        self.transform_jsx_member_expression_effect_only(node, false)
      }
      JSXElementName::ThisExpression(_node) => None,
    }
  }

  pub fn transform_jsx_element_name_need_val(
    &self,
    node: &'a JSXElementName<'a>,
  ) -> JSXElementName<'a> {
    match node {
      JSXElementName::Identifier(node) => JSXElementName::Identifier(self.clone_node(node)),
      JSXElementName::IdentifierReference(node) => JSXElementName::IdentifierReference(
        self.transform_identifier_reference_read(node, true).unwrap(),
      ),
      JSXElementName::MemberExpression(node) => {
        JSXElementName::MemberExpression(self.transform_jsx_member_expression_need_val(node))
      }
      JSXElementName::NamespacedName(_) | JSXElementName::ThisExpression(_) => {
        self.clone_node(node)
      }
    }
  }
}
