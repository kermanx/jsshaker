use crate::{analyzer::Analyzer, ast::AstKind2, scope::CfScopeKind, transformer::Transformer};
use oxc::{
  ast::ast::{IfStatement, Statement},
  span::GetSpan,
};

impl<'a> Analyzer<'a> {
  pub fn exec_if_statement(&mut self, node: &'a IfStatement) {
    let factory = self.factory;

    let test = self.exec_expression(&node.test).get_to_boolean(self);

    let (maybe_consequent, maybe_alternate) = match test.test_truthy() {
      Some(true) => (true, false),
      Some(false) => (false, true),
      None => (true, true),
    };

    let mut both_exit = true;
    let mut exit_target_inner = 0;
    let mut exit_target_outer = self.scoping.cf.stack.len();
    let mut acc_dep_1 = None;
    let mut acc_dep_2 = None;

    if maybe_consequent {
      self.push_if_like_branch_cf_scope(
        AstKind2::IfStatement(node),
        CfScopeKind::ExitBlocker(None),
        test,
        maybe_consequent,
        maybe_alternate,
        true,
        node.alternate.is_some(),
      );
      self.exec_statement(&node.consequent);
      let conditional_scope = self.pop_cf_scope_and_get_mut();
      if let CfScopeKind::ExitBlocker(Some(stopped_exit)) = &conditional_scope.kind {
        exit_target_inner = exit_target_inner.max(*stopped_exit);
        exit_target_outer = exit_target_outer.min(*stopped_exit);
      } else {
        both_exit = false;
      }
      acc_dep_1 = conditional_scope.deps.try_collect(factory);
    }
    if maybe_alternate {
      self.push_if_like_branch_cf_scope(
        AstKind2::IfStatement(node),
        CfScopeKind::ExitBlocker(None),
        test,
        maybe_consequent,
        maybe_alternate,
        false,
        true,
      );
      if let Some(alternate) = &node.alternate {
        self.exec_statement(alternate);
        let conditional_scope = self.pop_cf_scope_and_get_mut();
        if let CfScopeKind::ExitBlocker(Some(stopped_exit)) = &conditional_scope.kind {
          exit_target_inner = exit_target_inner.max(*stopped_exit);
          exit_target_outer = exit_target_outer.min(*stopped_exit);
        } else {
          both_exit = false;
        }
        acc_dep_2 = conditional_scope.deps.try_collect(factory);
      } else {
        self.pop_cf_scope();
        both_exit = false;
      }
    }

    let acc_dep = Some(self.dep((acc_dep_1, acc_dep_2)));
    if both_exit {
      if let Some(acc_dep) =
        self.exit_to_impl(exit_target_inner, self.scoping.cf.stack.len(), true, acc_dep)
      {
        self.exit_to_impl(exit_target_outer, exit_target_inner, false, acc_dep);
      }
    } else {
      self.exit_to_impl(exit_target_outer, self.scoping.cf.stack.len(), false, acc_dep);
    }
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_if_statement(&self, node: &'a IfStatement<'a>) -> Option<Statement<'a>> {
    let IfStatement { span, test, consequent, alternate } = node;

    let (need_test_val, maybe_consequent, maybe_alternate) =
      self.get_conditional_result(AstKind2::IfStatement(node));

    let test = self.transform_expression(test, need_test_val);
    let consequent = if maybe_consequent { self.transform_statement(consequent) } else { None };
    let alternate = if maybe_alternate {
      alternate.as_ref().and_then(|alt| self.transform_statement(alt))
    } else {
      None
    };

    if need_test_val {
      match (consequent, alternate) {
        (Some(consequent), alternate) => {
          Some(self.ast_builder.statement_if(*span, test.unwrap(), consequent, alternate))
        }
        (None, Some(alternate)) => Some(self.ast_builder.statement_if(
          *span,
          self.build_negate_expression(test.unwrap()),
          alternate,
          None,
        )),
        (None, None) => test.map(|test| self.ast_builder.statement_expression(test.span(), test)),
      }
    } else {
      let mut statements = self.ast_builder.vec();
      if let Some(test) = test {
        statements.push(self.ast_builder.statement_expression(test.span(), test));
      }
      if let Some(consequent) = consequent {
        statements.push(consequent);
      }
      if let Some(alternate) = alternate {
        statements.push(alternate);
      }

      if statements.is_empty() {
        None
      } else {
        Some(self.ast_builder.statement_block(*span, statements))
      }
    }
  }
}
