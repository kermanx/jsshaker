use oxc::{ast::ast::PropertyKey, span::GetSpan};

use crate::{analyzer::Analyzer, entity::Entity, transformer::Transformer};

impl<'a> Analyzer<'a> {
  pub fn exec_property_key(
    &mut self,
    node: &'a PropertyKey<'a>,
    dep: Option<Entity<'a>>,
  ) -> Entity<'a> {
    match node {
      PropertyKey::StaticIdentifier(node) => self.exec_identifier_name(node),
      PropertyKey::PrivateIdentifier(node) => self.exec_private_identifier(node),
      node => {
        self.exec_expression_with_dependency(node.to_expression(), dep).get_to_property_key(self)
      }
    }
  }
}

impl<'a> Transformer<'a> {
  /// Returns (computed, node)
  /// Notice that even if `need_val` is `false`, and the expression has side-effect, the transformed expression still evaluates to the original value.
  pub fn transform_property_key(
    &self,
    node: &'a PropertyKey<'a>,
    need_val: bool,
  ) -> Option<PropertyKey<'a>> {
    if self.declaration_only.get() {
      return need_val.then_some(PropertyKey::from(self.build_unused_expression(node.span())));
    }

    match node {
      // Reuse the node
      PropertyKey::StaticIdentifier(node) => need_val.then(|| {
        PropertyKey::StaticIdentifier(self.ast_builder.alloc(self.transform_identifier_name(node)))
      }),
      PropertyKey::PrivateIdentifier(private_identifier) => self
        .transform_private_identifier(private_identifier, need_val)
        .map(|k| PropertyKey::PrivateIdentifier(self.ast_builder.alloc(k))),
      _ => {
        let node = node.to_expression();
        if need_val || self.transform_expression(node, false).is_some() {
          let expr = self.transform_expression(node, true).unwrap();
          Some(PropertyKey::from(expr))
        } else {
          None
        }
      }
    }
  }
}
