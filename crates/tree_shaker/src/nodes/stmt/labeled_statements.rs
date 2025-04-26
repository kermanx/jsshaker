use oxc::ast::ast::{LabeledStatement, Statement};

use crate::{analyzer::Analyzer, ast::AstKind2, scope::CfScopeKind, transformer::Transformer};

impl<'a> Analyzer<'a> {
  pub fn declare_labeled_statement(&mut self, node: &'a LabeledStatement<'a>) {
    self.declare_statement(&node.body);
  }

  pub fn exec_labeled_statement(&mut self, node: &'a LabeledStatement<'a>) {
    self.push_cf_scope(CfScopeKind::Labeled(node), false);
    self.exec_statement(&node.body);
    self.pop_cf_scope();
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_labeled_statement(
    &self,
    node: &'a LabeledStatement<'a>,
  ) -> Option<Statement<'a>> {
    let LabeledStatement { span, label, body } = node;

    let body = self.transform_statement(body);

    if self.is_referred(AstKind2::LabeledStatement(node)) {
      body.map(|body| self.ast_builder.statement_labeled(*span, label.clone(), body))
    } else {
      body
    }
  }
}
