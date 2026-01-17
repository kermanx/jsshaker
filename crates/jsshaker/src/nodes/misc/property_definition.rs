use oxc::ast::{
  NONE,
  ast::{ClassElement, PropertyDefinition},
};

use crate::{Analyzer, entity::Entity, transformer::Transformer, utils::ast::AstKind2};

impl<'a> Analyzer<'a> {
  pub fn exec_property_definition(&mut self, node: &'a PropertyDefinition<'a>) -> Entity<'a> {
    self.factory.computed(
      if let Some(value) = &node.value {
        self.exec_expression(value)
      } else {
        self.factory.undefined
      },
      AstKind2::PropertyDefinition(node),
    )
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_property_definition(
    &self,
    node: &'a PropertyDefinition<'a>,
  ) -> Option<ClassElement<'a>> {
    let PropertyDefinition { r#type, span, decorators, key, value, computed, r#static, .. } = node;

    let need_value = self.is_included(AstKind2::PropertyDefinition(node));
    let value =
      if let Some(value) = value { self.transform_expression(value, need_value) } else { None };
    let key = self.transform_property_key(key, value.is_some())?;

    Some(self.ast.class_element_property_definition(
      *span,
      *r#type,
      self.clone_node(decorators),
      key,
      NONE,
      value,
      *computed,
      *r#static,
      false,
      false,
      false,
      false,
      false,
      None,
    ))
  }
}
