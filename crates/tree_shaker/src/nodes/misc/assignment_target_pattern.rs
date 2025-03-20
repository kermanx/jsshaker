use crate::{analyzer::Analyzer, ast::AstKind2, entity::Entity, transformer::Transformer};
use oxc::ast::ast::{ArrayAssignmentTarget, AssignmentTargetPattern, ObjectAssignmentTarget};

impl<'a> Analyzer<'a> {
  pub fn exec_assignment_target_pattern_write(
    &mut self,
    node: &'a AssignmentTargetPattern<'a>,
    value: Entity<'a>,
  ) {
    match node {
      AssignmentTargetPattern::ArrayAssignmentTarget(node) => {
        let (element_values, rest_value, dep) = value.destruct_as_array(
          self,
          AstKind2::ArrayAssignmentTarget(node),
          node.elements.len(),
          node.rest.is_some(),
        );

        self.push_dependent_cf_scope(dep);
        for (element, value) in node.elements.iter().zip(element_values) {
          if let Some(element) = element {
            self.exec_assignment_target_maybe_default(element, value);
          }
        }
        if let Some(rest) = &node.rest {
          self.exec_assignment_target_rest(rest, rest_value.unwrap());
        }
        self.pop_cf_scope();
      }
      AssignmentTargetPattern::ObjectAssignmentTarget(node) => {
        self.push_dependent_cf_scope(value.get_destructable(self, self.factory.empty_consumable));

        let is_nullish = value.test_nullish();
        if is_nullish != Some(false) {
          if is_nullish == Some(true) {
            self.throw_builtin_error("Cannot destructure nullish value");
          } else {
            self.may_throw();
          }
          value.consume(self);
          self.refer_dep(AstKind2::ObjectAssignmentTarget(node));
        }

        let mut enumerated = vec![];
        for property in &node.properties {
          enumerated.push(self.exec_assignment_target_property(property, value));
        }
        if let Some(rest) = &node.rest {
          let dep = AstKind2::ObjectAssignmentTarget(node);
          let init = self.exec_object_rest(dep, value, enumerated);
          self.exec_assignment_target_rest(rest, init);
        }

        self.pop_cf_scope();
      }
    }
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_assignment_target_pattern(
    &self,
    node: &'a AssignmentTargetPattern<'a>,
  ) -> Option<AssignmentTargetPattern<'a>> {
    match node {
      AssignmentTargetPattern::ArrayAssignmentTarget(node) => {
        let ArrayAssignmentTarget { span, elements, rest, trailing_comma } = node.as_ref();

        let is_referred = self.is_referred(AstKind2::ArrayAssignmentTarget(node));

        let mut transformed_elements = self.ast_builder.vec();
        for element in elements {
          transformed_elements.push(
            element
              .as_ref()
              .and_then(|element| self.transform_assignment_target_maybe_default(element, false)),
          );
        }

        let rest =
          rest.as_ref().and_then(|rest| self.transform_assignment_target_rest(rest, is_referred));

        if !is_referred && rest.is_none() {
          while transformed_elements.last().is_some_and(Option::is_none) {
            transformed_elements.pop();
          }
        }

        if !is_referred && transformed_elements.is_empty() && rest.is_none() {
          None
        } else {
          let trailing_comma = (transformed_elements.last().is_some_and(Option::is_none)
            && rest.is_none())
          .then_some(trailing_comma.unwrap_or_default());
          Some(self.ast_builder.assignment_target_pattern_array_assignment_target(
            *span,
            transformed_elements,
            rest,
            trailing_comma,
          ))
        }
      }
      AssignmentTargetPattern::ObjectAssignmentTarget(node) => {
        let ObjectAssignmentTarget { span, properties, rest } = node.as_ref();

        let is_referred = self.is_referred(AstKind2::ObjectAssignmentTarget(node));

        let rest = rest.as_ref().and_then(|rest| {
          self.transform_assignment_target_rest(
            rest,
            self.is_referred(AstKind2::ObjectAssignmentTarget(node)),
          )
        });

        let mut transformed_properties = self.ast_builder.vec();
        for property in properties {
          if let Some(property) = self.transform_assignment_target_property(property) {
            transformed_properties.push(property);
          }
        }
        if !is_referred && transformed_properties.is_empty() && rest.is_none() {
          None
        } else {
          Some(self.ast_builder.assignment_target_pattern_object_assignment_target(
            *span,
            transformed_properties,
            rest,
          ))
        }
      }
    }
  }
}
