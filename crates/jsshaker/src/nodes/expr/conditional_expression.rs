use oxc::ast::ast::{ConditionalExpression, Expression, LogicalOperator};

use crate::{
  analyzer::Analyzer, ast::AstKind2, build_effect, entity::Entity, scope::CfScopeKind,
  transformer::Transformer,
};

impl<'a> Analyzer<'a> {
  pub fn exec_conditional_expression(&mut self, node: &'a ConditionalExpression<'a>) -> Entity<'a> {
    let test = self.exec_expression(&node.test);

    let (maybe_true, maybe_false) = match test.test_truthy() {
      Some(true) => (true, false),
      Some(false) => (false, true),
      None => (true, true),
    };

    let exec_consequent = move |analyzer: &mut Analyzer<'a>| {
      let conditional_dep = analyzer.push_if_like_branch_cf_scope(
        AstKind2::ConditionalExpression(node),
        CfScopeKind::Indeterminate,
        test,
        maybe_true,
        maybe_false,
        true,
        true,
      );
      let value = analyzer.exec_expression(&node.consequent);
      analyzer.pop_cf_scope();
      analyzer.factory.computed(value, conditional_dep)
    };

    let exec_alternate = move |analyzer: &mut Analyzer<'a>| {
      let conditional_dep = analyzer.push_if_like_branch_cf_scope(
        AstKind2::ConditionalExpression(node),
        CfScopeKind::Indeterminate,
        test,
        maybe_true,
        maybe_false,
        false,
        true,
      );
      let value = analyzer.exec_expression(&node.alternate);
      analyzer.pop_cf_scope();
      analyzer.factory.computed(value, conditional_dep)
    };

    match (maybe_true, maybe_false) {
      (true, false) => exec_consequent(self),
      (false, true) => exec_alternate(self),
      (true, true) => {
        let v1 = exec_consequent(self);
        let v2 = exec_alternate(self);
        self.factory.union((v1, v2))
      }
      _ => unreachable!("Conditional expression should have at least one possible branch"),
    }
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_conditional_expression(
    &self,
    node: &'a ConditionalExpression<'a>,
    need_val: bool,
  ) -> Option<Expression<'a>> {
    let ConditionalExpression { span, test, consequent, alternate } = node;

    let (need_test_val, maybe_true, maybe_false) =
      self.get_conditional_result(AstKind2::ConditionalExpression(node));

    let test = self.transform_expression(test, need_test_val);
    let consequent = maybe_true.then(|| self.transform_expression(consequent, need_val)).flatten();
    let alternate = maybe_false.then(|| self.transform_expression(alternate, need_val)).flatten();

    if need_test_val {
      let test = test.unwrap();

      match (consequent, alternate) {
        (Some(consequent), Some(alternate)) => {
          Some(self.ast_builder.expression_conditional(*span, test, consequent, alternate))
        }
        (Some(consequent), None) => {
          Some(self.ast_builder.expression_logical(*span, test, LogicalOperator::And, consequent))
        }
        (None, Some(alternate)) => {
          Some(self.ast_builder.expression_logical(*span, test, LogicalOperator::Or, alternate))
        }
        (None, None) => unreachable!("node {}, {} @{:?}", maybe_true, maybe_false, span),
      }
    } else {
      build_effect!(self.ast_builder, *span, test, consequent, alternate)
    }
  }
}
