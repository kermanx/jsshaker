use oxc::ast::ast::{AssignmentExpression, AssignmentOperator, Expression};

use crate::{
  analyzer::Analyzer, ast::AstKind2, build_effect, entity::Entity, transformer::Transformer,
};

impl<'a> Analyzer<'a> {
  pub fn exec_assignment_expression(&mut self, node: &'a AssignmentExpression<'a>) -> Entity<'a> {
    if node.operator == AssignmentOperator::Assign {
      let rhs = self.exec_expression(&node.right);
      self.exec_assignment_target_write(&node.left, rhs, None);
      rhs
    } else if node.operator.is_logical() {
      let (left, cache) = self.exec_assignment_target_read(&node.left);
      let left_prim = left.coerce_primitive(self);

      let (maybe_left, maybe_right) = match &node.operator {
        AssignmentOperator::LogicalAnd => match left_prim.test_truthy() {
          Some(true) => (false, true),
          Some(false) => (true, false),
          None => (true, true),
        },
        AssignmentOperator::LogicalOr => match left_prim.test_truthy() {
          Some(true) => (true, false),
          Some(false) => (false, true),
          None => (true, true),
        },
        AssignmentOperator::LogicalNullish => match left_prim.test_nullish() {
          Some(true) => (false, true),
          Some(false) => (true, false),
          None => (true, true),
        },
        _ => unreachable!(),
      };

      let forward_left = |analyzer: &mut Analyzer<'a>| {
        analyzer.forward_logical_left_val(
          AstKind2::LogicalAssignmentExpressionLeft(node),
          left,
          left_prim,
          maybe_left,
          maybe_right,
        )
      };
      let exec_right = |analyzer: &mut Analyzer<'a>| {
        let conditional_dep = analyzer.push_logical_right_cf_scope(
          AstKind2::LogicalAssignmentExpressionLeft(node),
          left_prim,
          maybe_left,
          maybe_right,
        );

        let val = analyzer.factory.computed(analyzer.exec_expression(&node.right), conditional_dep);

        analyzer.pop_cf_scope();

        val
      };

      let value = match (maybe_left, maybe_right) {
        (false, true) => exec_right(self),
        (true, false) => forward_left(self),
        (true, true) => {
          let left = forward_left(self);
          let right = exec_right(self);
          self.factory.logical_result(left, right, node.operator.to_logical_operator().unwrap())
        }
        (false, false) => unreachable!(),
      };

      if maybe_right {
        self.exec_assignment_target_write(&node.left, value, cache);
      }

      value
    } else {
      let (lhs, cache) = self.exec_assignment_target_read(&node.left);
      let rhs = self.exec_expression(&node.right);
      let value = self.op_binary(node.operator.to_binary_operator().unwrap(), lhs, rhs);
      self.exec_assignment_target_write(&node.left, value, cache);
      value
    }
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_assignment_expression(
    &self,
    node: &'a AssignmentExpression<'a>,
    need_val: bool,
  ) -> Option<Expression<'a>> {
    let AssignmentExpression { span, operator, left, right } = node;

    let (left_is_empty, transformed_left) =
      self.transform_assignment_target_write(left, false, false);
    let transformed_right = self.transform_expression(right, need_val || !left_is_empty);

    match (transformed_left, transformed_right) {
      (Some(left), right) => {
        debug_assert!(!left_is_empty);
        Some(self.ast.expression_assignment(
          *span,
          if operator.is_logical() {
            let (_, maybe_left, _) =
              self.get_conditional_result(AstKind2::LogicalAssignmentExpressionLeft(node), false);

            if maybe_left { *operator } else { AssignmentOperator::Assign }
          } else {
            *operator
          },
          left,
          right.unwrap(),
        ))
      }
      (None, Some(right)) => {
        if need_val && *operator != AssignmentOperator::Assign {
          if operator.is_logical() {
            let (need_left_test_val, maybe_left, maybe_right) =
              self.get_conditional_result(AstKind2::LogicalAssignmentExpressionLeft(node), false);

            let maybe_left = (need_val && maybe_left) || need_left_test_val;
            let left = self.transform_assignment_target_read(left, maybe_left);
            let right = maybe_right.then_some(right);

            if need_left_test_val {
              let left = left.unwrap();
              if let Some(right) = right {
                Some(self.ast.expression_logical(
                  *span,
                  left,
                  operator.to_logical_operator().unwrap(),
                  right,
                ))
              } else {
                Some(left)
              }
            } else {
              build_effect!(self.ast, *span, left, right)
            }
          } else {
            let left = self.transform_assignment_target_read(left, true).unwrap();
            Some(self.ast.expression_binary(
              *span,
              left,
              operator.to_binary_operator().unwrap(),
              right,
            ))
          }
        } else {
          Some(right)
        }
      }
      (None, None) => None,
    }
  }
}
