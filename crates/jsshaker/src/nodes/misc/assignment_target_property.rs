use oxc::{
  ast::ast::{
    AssignmentTargetProperty, AssignmentTargetPropertyIdentifier, AssignmentTargetPropertyProperty,
    SimpleAssignmentTarget,
  },
  span::{GetSpan, SPAN},
};

use crate::{analyzer::Analyzer, ast::AstKind2, entity::Entity, transformer::Transformer};

impl<'a> Analyzer<'a> {
  /// Returns the key
  pub fn exec_assignment_target_property(
    &mut self,
    node: &'a AssignmentTargetProperty<'a>,
    value: Entity<'a>,
  ) -> Entity<'a> {
    let dep = AstKind2::AssignmentTargetProperty(node);
    match node {
      AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(node) => {
        let key = self.exec_identifier_reference_as_key(&node.binding);

        let value = value.get_property(self, dep, key);

        let value =
          if let Some(init) = &node.init { self.exec_with_default(init, value) } else { value };

        self.exec_identifier_reference_write(&node.binding, value);

        key
      }
      AssignmentTargetProperty::AssignmentTargetPropertyProperty(node) => {
        let key = self.exec_property_key(&node.name, Some(value));
        let value = value.get_property(self, dep, key);
        self.exec_assignment_target_maybe_default(&node.binding, value);
        key
      }
    }
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_assignment_target_property(
    &self,
    node: &'a AssignmentTargetProperty<'a>,
  ) -> Option<AssignmentTargetProperty<'a>> {
    let need_binding = self.is_referred(AstKind2::AssignmentTargetProperty(node));
    match node {
      AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(node) => {
        let AssignmentTargetPropertyIdentifier { span, binding, init } = node.as_ref();

        let binding_write = self.transform_identifier_reference_write(binding);
        let binding_key = self.transform_identifier_reference_as_key(binding);
        let init = if let Some(init) = init {
          self.transform_with_default(init, binding_write.is_some())
        } else {
          None
        };

        if need_binding && binding_write.is_none() {
          Some(self.ast_builder.assignment_target_property_assignment_target_property_property(
            *span,
            binding_key,
            if let Some(init) = init {
              self.ast_builder.assignment_target_maybe_default_assignment_target_with_default(
                *span,
                self.build_unused_assignment_target(SPAN),
                init,
              )
            } else {
              self.build_unused_assignment_target(SPAN).into()
            },
            false,
          ))
        } else if binding_write.is_some() || init.is_some() {
          let binding_write = binding_write.map_or_else(
            || self.build_unused_assignment_target(SPAN),
            |b| SimpleAssignmentTarget::AssignmentTargetIdentifier(b).into(),
          );
          Some(self.ast_builder.assignment_target_property_assignment_target_property_property(
            *span,
            binding_key,
            if let Some(init) = init {
              self.ast_builder.assignment_target_maybe_default_assignment_target_with_default(
                *span,
                binding_write,
                init,
              )
            } else {
              binding_write.into()
            },
            false,
          ))
        } else {
          None
        }
      }
      AssignmentTargetProperty::AssignmentTargetPropertyProperty(node) => {
        let AssignmentTargetPropertyProperty { span, name, binding, computed } = node.as_ref();

        let name_span = name.span();
        let binding = self.transform_assignment_target_maybe_default(binding, need_binding);
        if let Some(binding) = binding {
          let name = self.transform_property_key(name, true).unwrap();
          Some(self.ast_builder.assignment_target_property_assignment_target_property_property(
            *span, name, binding, *computed,
          ))
        } else {
          self.transform_property_key(name, false).map(|name| {
            self.ast_builder.assignment_target_property_assignment_target_property_property(
              *span,
              name,
              self.build_unused_assignment_target(name_span).into(),
              *computed,
            )
          })
        }
      }
    }
  }
}
