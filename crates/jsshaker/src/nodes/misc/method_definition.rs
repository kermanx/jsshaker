use oxc::{
  ast::{
    NONE,
    ast::{ClassElement, Function, MethodDefinition, MethodDefinitionKind, PropertyDefinitionType},
  },
  span::GetSpan,
};

use crate::transformer::Transformer;

impl<'a> Transformer<'a> {
  pub fn transform_method_definition(
    &self,
    node: &'a MethodDefinition<'a>,
  ) -> Option<ClassElement<'a>> {
    let MethodDefinition {
      r#type,
      span,
      decorators,
      key,
      value,
      kind,
      computed,
      r#static,
      r#override,
      optional,
      accessibility,
    } = node;

    if let Some(mut transformed_value) = self.transform_function(value, false) {
      let key = if node.kind.is_constructor() {
        self.clone_node(key)
      } else {
        self.transform_property_key(key, true).unwrap()
      };

      if *kind == MethodDefinitionKind::Set {
        self.patch_method_definition_params(value, &mut transformed_value);
      }

      Some(self.ast.class_element_method_definition(
        *span,
        *r#type,
        self.transform_decorators(decorators),
        key,
        transformed_value,
        *kind,
        *computed,
        *r#static,
        *r#override,
        *optional,
        *accessibility,
      ))
    } else {
      let key = self.transform_property_key(key, false);
      key.map(|key| {
        self.ast.class_element_property_definition(
          *span,
          PropertyDefinitionType::PropertyDefinition,
          self.ast.vec(),
          key,
          NONE,
          None,
          true,
          *r#static,
          false,
          false,
          false,
          false,
          false,
          None,
        )
      })
    }
  }

  /// It is possible that `set a(param) {}` has been optimized to `set a() {}`.
  /// This function patches the parameter list if it is empty.
  pub fn patch_method_definition_params(
    &self,
    original_node: &'a Function<'a>,
    transformed_node: &mut Function<'a>,
  ) {
    if !transformed_node.params.has_parameter() {
      let span = original_node.span;
      let original_param = &original_node.params.items[0];
      transformed_node.params.items.push(self.ast.formal_parameter(
        span,
        self.ast.vec(),
        self.build_unused_binding_pattern(span),
        NONE,
        if self.config.preserve_function_length
          && let Some(initializer) = &original_param.initializer
        {
          Some(self.build_unused_expression(initializer.span()))
        } else {
          None
        },
        false,
        None,
        false,
        false,
      ));
    }
  }
}
