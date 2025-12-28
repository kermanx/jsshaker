use oxc::{
  allocator,
  ast::{
    NONE,
    ast::{Expression, JSXClosingElement, JSXElement, JSXOpeningElement, PropertyKind},
  },
};

use crate::{analyzer::Analyzer, build_effect, entity::Entity, transformer::Transformer};

impl<'a> Analyzer<'a> {
  pub fn exec_jsx_element(&mut self, node: &'a JSXElement<'a>) -> Entity<'a> {
    let tag = self.exec_jsx_element_name(&node.opening_element.name);
    let attributes = self.exec_jsx_attributes(&node.opening_element);
    let children = self.exec_jsx_children(&node.children);
    let key_children = *self.builtins.react_data.key_children.get_or_insert_with(|| {
      self.factory.mangable_string("children", self.mangler.new_constant_atom("children"))
    });
    attributes.init_property(self, PropertyKind::Init, key_children, children, true);
    self.factory.react_element(tag, attributes.into())
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_jsx_element(
    &self,
    node: &'a JSXElement<'a>,
    need_val: bool,
  ) -> Option<Expression<'a>> {
    if need_val {
      Some(Expression::JSXElement(self.transform_jsx_element_need_val(node)))
    } else {
      self.transform_jsx_element_effect_only(node)
    }
  }

  pub fn transform_jsx_element_effect_only(
    &self,
    node: &'a JSXElement<'a>,
  ) -> Option<Expression<'a>> {
    let JSXElement { span, opening_element, children, .. } = node;

    build_effect!(
      self.ast,
      *span,
      vec![self.transform_jsx_element_name_effect_only(&opening_element.name)],
      self.transform_jsx_attributes_effect_only(&opening_element.attributes),
      self.transform_jsx_children_effect_only(children),
    )
  }

  pub fn transform_jsx_element_need_val(
    &self,
    node: &'a JSXElement<'a>,
  ) -> allocator::Box<'a, JSXElement<'a>> {
    let JSXElement { span, opening_element, closing_element, children } = node;

    let name = self.transform_jsx_element_name_need_val(&opening_element.name);

    let closing_element = closing_element.as_ref().map(|closing_element| {
      let JSXClosingElement { span, .. } = closing_element.as_ref();

      self.ast.jsx_closing_element(*span, self.clone_node(&name))
    });

    self.ast.alloc_jsx_element(
      *span,
      {
        let JSXOpeningElement { span, name, attributes, .. } = opening_element.as_ref();

        self.ast.jsx_opening_element(
          *span,
          self.clone_node(name),
          NONE,
          self.transform_jsx_attributes_need_val(attributes),
        )
      },
      self.transform_jsx_children_need_val(children),
      closing_element,
    )
  }
}
