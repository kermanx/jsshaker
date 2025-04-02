use oxc::ast::ast::AssignmentTargetRest;

use crate::{analyzer::Analyzer, entity::Entity, transformer::Transformer};

impl<'a> Analyzer<'a> {
  pub fn exec_assignment_target_rest(
    &mut self,
    node: &'a AssignmentTargetRest<'a>,
    value: Entity<'a>,
  ) {
    self.exec_assignment_target_write(&node.target, value, None)
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_assignment_target_rest(
    &self,
    node: &'a AssignmentTargetRest<'a>,
    need_binding: bool,
  ) -> Option<AssignmentTargetRest<'a>> {
    let AssignmentTargetRest { span, target } = node;

    let target = self.transform_assignment_target_write(target, need_binding, true);

    target.1.map(|target| self.ast_builder.assignment_target_rest(*span, target))
  }
}
