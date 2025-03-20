use crate::{analyzer::Analyzer, ast::AstKind2, scope::CfScopeKind, transformer::Transformer};
use oxc::{
  ast::ast::{ForInStatement, Statement},
  span::GetSpan,
};

impl<'a> Analyzer<'a> {
  pub fn exec_for_in_statement(&mut self, node: &'a ForInStatement<'a>) {
    let right = self.exec_expression(&node.right);

    if let Some(keys) = right.get_own_keys(self) {
      let dep = right.get_destructable(self, AstKind2::ForInStatement(node));
      self.push_cf_scope_with_deps(CfScopeKind::LoopBreak, vec![dep], Some(false));
      for (definite, key) in keys {
        self.push_cf_scope_with_deps(
          CfScopeKind::LoopContinue,
          vec![self.factory.always_mangable_dep(key)],
          if definite { Some(false) } else { None },
        );
        self.push_variable_scope();

        self.declare_for_statement_left(&node.left);
        self.init_for_statement_left(&node.left, key);

        self.exec_statement(&node.body);

        self.pop_variable_scope();
        self.pop_cf_scope();

        if self.cf_scope().must_exited() {
          break;
        }
      }
      self.pop_cf_scope();
    } else {
      let dep = self.consumable((AstKind2::ForInStatement(node), right));
      self.push_cf_scope_with_deps(CfScopeKind::LoopBreak, vec![dep], Some(false));
      self.exec_loop(move |analyzer| {
        analyzer.push_cf_scope_with_deps(CfScopeKind::LoopContinue, vec![], None);
        analyzer.push_variable_scope();

        analyzer.declare_for_statement_left(&node.left);
        analyzer.init_for_statement_left(&node.left, analyzer.factory.unknown_string);

        analyzer.exec_statement(&node.body);

        analyzer.pop_variable_scope();
        analyzer.pop_cf_scope();
      });
      self.pop_cf_scope();
    }
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_for_in_statement(&self, node: &'a ForInStatement<'a>) -> Option<Statement<'a>> {
    let ForInStatement { span, left, right, body, .. } = node;

    let need_loop = self.is_referred(AstKind2::ForInStatement(node));

    let left_span = left.span();
    let body_span = body.span();

    let left = self.transform_for_statement_left(left);
    let body = self.transform_statement(body);

    if !need_loop || (left.is_none() && body.is_none()) {
      return self
        .transform_expression(right, false)
        .map(|expr| self.ast_builder.statement_expression(*span, expr));
    }

    let right = self.transform_expression(right, true).unwrap();

    Some(self.ast_builder.statement_for_in(
      *span,
      left.unwrap_or_else(|| self.build_unused_for_statement_left(left_span)),
      right,
      body.unwrap_or_else(|| self.ast_builder.statement_empty(body_span)),
    ))
  }
}
