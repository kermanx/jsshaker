use oxc::{
  ast::ast::{ArrayPattern, AssignmentPattern, BindingPattern, BindingProperty, ObjectPattern},
  span::GetSpan,
};

use crate::{
  Analyzer,
  ast::{AstKind2, DeclarationKind},
  dep::DepAtom,
  entity::Entity,
  transformer::Transformer,
};

impl<'a> Analyzer<'a> {
  pub fn declare_binding_pattern(
    &mut self,
    node: &'a BindingPattern<'a>,
    exporting: Option<DepAtom>,
    kind: DeclarationKind,
  ) {
    match node {
      BindingPattern::BindingIdentifier(node) => {
        self.declare_binding_identifier(node, exporting, kind);
      }
      BindingPattern::ObjectPattern(node) => {
        for property in &node.properties {
          self.declare_binding_pattern(&property.value, exporting, kind);
        }
        if let Some(rest) = &node.rest {
          self.declare_binding_rest_element(rest, exporting, kind);
        }
      }
      BindingPattern::ArrayPattern(node) => {
        for element in node.elements.iter().flatten() {
          self.declare_binding_pattern(element, exporting, kind);
        }
        if let Some(rest) = &node.rest {
          self.declare_binding_rest_element(rest, exporting, kind);
        }
      }
      BindingPattern::AssignmentPattern(node) => {
        self.declare_binding_pattern(&node.left, exporting, kind);
      }
    }
  }

  pub fn init_binding_pattern(&mut self, node: &'a BindingPattern<'a>, init: Option<Entity<'a>>) {
    match node {
      BindingPattern::BindingIdentifier(node) => {
        self.init_binding_identifier(node, init);
      }
      BindingPattern::ObjectPattern(node) => {
        let init = init.unwrap_or_else(|| {
          self.throw_builtin_error("Missing initializer in destructuring declaration");
          self.factory.unknown
        });

        self.push_dependent_cf_scope(init.get_shallow_dep(self));
        let is_nullish = init.test_nullish();
        if is_nullish != Some(false) {
          if is_nullish == Some(true) {
            self.throw_builtin_error("Cannot destructure nullish value");
          }
          if self.config.preserve_exceptions {
            self.consume((init, AstKind2::ObjectPattern(node.as_ref())));
          }
        }

        let mut enumerated = vec![];
        for property in &node.properties {
          let dep = AstKind2::BindingProperty(property);

          let key = self.exec_property_key(&property.key, Some(init));

          enumerated.push(key);
          let init = init.get_property(self, dep, key);
          self.init_binding_pattern(&property.value, Some(init));
        }
        if let Some(rest) = &node.rest {
          let dep = AstKind2::BindingRestElement(rest.as_ref());
          let init = self.exec_object_rest(dep, init, enumerated);
          self.init_binding_rest_element(rest, init);
        }
        self.pop_cf_scope();
      }
      BindingPattern::ArrayPattern(node) => {
        let init = init.unwrap_or_else(|| {
          self.throw_builtin_error("Missing initializer in destructuring declaration");
          self.factory.unknown
        });

        let (element_values, rest_value, dep) = init.destruct_as_array(
          self,
          AstKind2::ArrayPattern(node),
          node.elements.len(),
          node.rest.is_some(),
        );

        let is_simple = node
          .elements
          .iter()
          .all(|element| matches!(element, None | Some(BindingPattern::BindingIdentifier(_))))
          && node
            .rest
            .as_ref()
            .is_none_or(|rest| matches!(rest.argument, BindingPattern::BindingIdentifier(_)));

        if !is_simple {
          self.push_dependent_cf_scope(dep);
        }

        for (element, value) in node.elements.iter().zip(element_values) {
          if let Some(element) = element {
            self.init_binding_pattern(element, Some(value));
          }
        }
        if let Some(rest) = &node.rest {
          self.init_binding_rest_element(rest, rest_value.unwrap());
        }

        if !is_simple {
          self.pop_cf_scope();
        }
      }
      BindingPattern::AssignmentPattern(node) => {
        let binding_val = self.exec_with_default(&node.right, init.unwrap());
        self.init_binding_pattern(&node.left, Some(binding_val));
      }
    }
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_binding_pattern(
    &self,
    node: &'a BindingPattern<'a>,
    need_binding: bool,
  ) -> Option<BindingPattern<'a>> {
    let span = node.span();

    match node {
      BindingPattern::BindingIdentifier(node) => {
        let result = self
          .transform_binding_identifier(node)
          .map(|identifier| BindingPattern::BindingIdentifier(self.ast.alloc(identifier)));

        if need_binding {
          Some(result.unwrap_or_else(|| self.build_unused_binding_pattern(span)))
        } else {
          result
        }
      }
      BindingPattern::ObjectPattern(node) => {
        let ObjectPattern { span, properties, rest } = node.as_ref();

        let need_binding =
          need_binding || self.is_deoptimized(AstKind2::ObjectPattern(node.as_ref()));

        let rest = rest.as_ref().and_then(|rest| {
          self.transform_binding_rest_element(
            rest,
            self.is_deoptimized(AstKind2::BindingRestElement(rest.as_ref())),
          )
        });

        let mut transformed_properties = self.ast.vec();
        for property in properties {
          let dep = AstKind2::BindingProperty(property);
          let need_property = self.is_deoptimized(dep);

          let BindingProperty { span, key, value, shorthand, computed } = property;

          let transformed_key = self.transform_property_key(key, need_property);
          let shorthand = *shorthand
            && transformed_key.as_ref().is_none_or(|transformed| transformed.name() == key.name());
          if shorthand && matches!(value, BindingPattern::BindingIdentifier(_)) {
            if self.transform_binding_pattern(value, false).is_some()
              || need_property
              || transformed_key.is_some()
            {
              transformed_properties.push(self.clone_node(property));
            }
          } else {
            let value = self.transform_binding_pattern(value, transformed_key.is_some());
            if let Some(value) = value {
              let transformed_key =
                transformed_key.unwrap_or_else(|| self.transform_property_key(key, true).unwrap());
              transformed_properties.push(self.ast.binding_property(
                *span,
                transformed_key,
                value,
                shorthand,
                *computed,
              ));
            }
          }
        }

        if !need_binding && transformed_properties.is_empty() && rest.is_none() {
          None
        } else {
          Some(self.ast.binding_pattern_object_pattern(*span, transformed_properties, rest))
        }
      }
      BindingPattern::ArrayPattern(node) => {
        let ArrayPattern { span, elements, rest } = node.as_ref();

        let deoptimized = self.is_deoptimized(AstKind2::ArrayPattern(node));

        let mut transformed_elements = self.ast.vec();
        for element in elements {
          transformed_elements.push(
            element.as_ref().and_then(|element| self.transform_binding_pattern(element, false)),
          );
        }

        let rest =
          rest.as_ref().and_then(|rest| self.transform_binding_rest_element(rest, deoptimized));

        if !deoptimized && rest.is_none() {
          while transformed_elements.last().is_some_and(Option::is_none) {
            transformed_elements.pop();
          }
        }

        if !need_binding && !deoptimized && transformed_elements.is_empty() && rest.is_none() {
          None
        } else {
          Some(self.ast.binding_pattern_array_pattern(*span, transformed_elements, rest))
        }
      }
      BindingPattern::AssignmentPattern(node) => {
        let AssignmentPattern { span, left, right } = node.as_ref();

        let left_span = left.span();
        let transformed_left = self.transform_binding_pattern(left, false);
        let transformed_right = if self.declaration_only.get() {
          None
        } else {
          self.transform_with_default(right, transformed_left.is_some())
        };

        if let Some(right) = transformed_right {
          Some(self.ast.binding_pattern_assignment_pattern(
            *span,
            transformed_left.unwrap_or(self.build_unused_binding_pattern(left_span)),
            right,
          ))
        } else if need_binding {
          Some(
            transformed_left.unwrap_or_else(|| self.transform_binding_pattern(left, true).unwrap()),
          )
        } else {
          transformed_left
        }
      }
    }
  }
}
