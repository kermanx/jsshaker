use oxc::{
  ast::ast::{AssignmentTargetMaybeDefault, AssignmentTargetWithDefault},
  span::GetSpan,
};

use crate::{analyzer::Analyzer, entity::Entity, transformer::Transformer};

impl<'a> Analyzer<'a> {
  pub fn exec_assignment_target_maybe_default(
    &mut self,
    node: &'a AssignmentTargetMaybeDefault<'a>,
    value: Entity<'a>,
  ) {
    match node {
      AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(node) => {
        let value = self.exec_with_default(&node.init, value);
        self.exec_assignment_target_write(&node.binding, value, None);
      }
      _ => self.exec_assignment_target_write(node.to_assignment_target(), value, None),
    }
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_assignment_target_maybe_default(
    &self,
    node: &'a AssignmentTargetMaybeDefault<'a>,
    need_binding: bool,
  ) -> Option<AssignmentTargetMaybeDefault<'a>> {
    match node {
      AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(node) => {
        let AssignmentTargetWithDefault { span, binding, init } = node.as_ref();

        let binding_span = binding.span();
        let (binding_is_empty, binding) =
          self.transform_assignment_target_write(binding, need_binding, false);
        let init = self.transform_with_default(init, !binding_is_empty);

        if let Some(init) = init {
          Some(self.ast_builder.assignment_target_maybe_default_assignment_target_with_default(
            *span,
            binding.unwrap_or(self.build_unused_assignment_target(binding_span)),
            init,
          ))
        } else {
          binding.map(AssignmentTargetMaybeDefault::from)
        }
      }
      _ => self
        .transform_assignment_target_write(node.to_assignment_target(), need_binding, false)
        .1
        .map(AssignmentTargetMaybeDefault::from),
    }
  }
}
