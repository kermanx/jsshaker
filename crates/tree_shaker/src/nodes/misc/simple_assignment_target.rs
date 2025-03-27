use oxc::ast::{
  ast::{Expression, SimpleAssignmentTarget},
  match_member_expression,
};

use crate::{analyzer::Analyzer, entity::Entity, transformer::Transformer};

impl<'a> Analyzer<'a> {
  pub fn exec_simple_assignment_target_read(
    &mut self,
    node: &'a SimpleAssignmentTarget<'a>,
  ) -> (Entity<'a>, Option<(Entity<'a>, Entity<'a>)>) {
    match node {
      match_member_expression!(SimpleAssignmentTarget) => {
        let (value, cache) = self.exec_member_expression_read(node.to_member_expression(), true);
        (value, Some(cache))
      }
      SimpleAssignmentTarget::AssignmentTargetIdentifier(node) => {
        (self.exec_identifier_reference_read(node), None)
      }
      _ => unreachable!(),
    }
  }

  pub fn exec_simple_assignment_target_write(
    &mut self,
    node: &'a SimpleAssignmentTarget<'a>,
    value: Entity<'a>,
    cache: Option<(Entity<'a>, Entity<'a>)>,
  ) {
    match node {
      match_member_expression!(SimpleAssignmentTarget) => {
        self.exec_member_expression_write(node.to_member_expression(), value, cache)
      }
      SimpleAssignmentTarget::AssignmentTargetIdentifier(node) => {
        self.exec_identifier_reference_write(node, value)
      }
      _ => unreachable!(),
    }
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_simple_assignment_target_read(
    &self,
    node: &'a SimpleAssignmentTarget<'a>,
    need_val: bool,
  ) -> Option<Expression<'a>> {
    match node {
      match_member_expression!(SimpleAssignmentTarget) => {
        self.transform_member_expression_read(node.to_member_expression(), need_val)
      }
      SimpleAssignmentTarget::AssignmentTargetIdentifier(node) => {
        self.transform_identifier_reference_read(node, need_val).map(Expression::Identifier)
      }
      _ => unreachable!(),
    }
  }

  pub fn transform_simple_assignment_target_write(
    &self,
    node: &'a SimpleAssignmentTarget<'a>,
  ) -> Option<SimpleAssignmentTarget<'a>> {
    match node {
      match_member_expression!(SimpleAssignmentTarget) => self
        .transform_member_expression_write(node.to_member_expression())
        .map(SimpleAssignmentTarget::from),
      SimpleAssignmentTarget::AssignmentTargetIdentifier(node) => self
        .transform_identifier_reference_write(node)
        .map(SimpleAssignmentTarget::AssignmentTargetIdentifier),
      _ => unreachable!(),
    }
  }
}
