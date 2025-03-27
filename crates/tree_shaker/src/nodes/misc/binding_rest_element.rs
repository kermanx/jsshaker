use oxc::ast::ast::BindingRestElement;

use crate::{analyzer::Analyzer, ast::DeclarationKind, entity::Entity, transformer::Transformer};

impl<'a> Analyzer<'a> {
  pub fn declare_binding_rest_element(
    &mut self,
    node: &'a BindingRestElement<'a>,
    exporting: bool,
    kind: DeclarationKind,
  ) {
    self.declare_binding_pattern(&node.argument, exporting, kind);
  }

  pub fn init_binding_rest_element(&mut self, node: &'a BindingRestElement<'a>, init: Entity<'a>) {
    self.init_binding_pattern(&node.argument, Some(init));
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_binding_rest_element(
    &self,
    node: &'a BindingRestElement<'a>,
    need_binding: bool,
  ) -> Option<BindingRestElement<'a>> {
    let BindingRestElement { span, argument } = node;

    let argument = self.transform_binding_pattern(argument, need_binding);

    argument.map(|argument| self.ast_builder.binding_rest_element(*span, argument))
  }
}
