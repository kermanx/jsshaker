use oxc::{
  ast::ast::{ForStatement, ForStatementInit, Statement},
  span::GetSpan,
};

use crate::{analyzer::Analyzer, ast::AstKind2, scope::CfScopeKind, transformer::Transformer};

impl<'a> Analyzer<'a> {
  pub fn exec_for_statement(&mut self, node: &'a ForStatement<'a>) {
    if let Some(init) = &node.init {
      match init {
        ForStatementInit::VariableDeclaration(node) => {
          self.declare_variable_declaration(node, None);
          self.init_variable_declaration(node, None);
        }
        node => {
          self.exec_expression(node.to_expression());
        }
      }
    }

    let dep = if let Some(test) = &node.test {
      let test = self.exec_expression(test).coerce_primitive(self);
      if test.test_truthy() == Some(false) {
        return;
      }
      self.dep((AstKind2::ForStatement(node), test))
    } else {
      self.dep(AstKind2::ForStatement(node))
    };

    self.push_cf_scope_with_deps(CfScopeKind::LoopBreak, self.factory.vec1(dep), false);
    self.exec_loop(move |analyzer| {
      if analyzer.cf_scope().must_exited() {
        return;
      }

      analyzer.push_cf_scope(CfScopeKind::LoopContinue, true);
      analyzer.exec_statement(&node.body);
      if let Some(update) = &node.update {
        analyzer.exec_expression(update);
      }
      analyzer.pop_cf_scope();

      if let Some(test) = &node.test {
        let test = analyzer.exec_expression(test).coerce_primitive(analyzer);
        let dep = analyzer.dep(test);
        analyzer.cf_scope_mut().push_dep(dep);
        let truthy = test.test_truthy();
        if truthy != Some(true) {
          analyzer.break_to_label(None, truthy.is_none());
        }
      }
    });
    self.pop_cf_scope();
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_for_statement(&self, node: &'a ForStatement<'a>) -> Option<Statement<'a>> {
    let ForStatement { span, init, test, update, body, .. } = node;

    if self.is_included(AstKind2::ForStatement(node)) {
      let init = init.as_ref().and_then(|init| match init {
        ForStatementInit::VariableDeclaration(node) => {
          self.transform_variable_declaration(node).map(ForStatementInit::VariableDeclaration)
        }
        node => self.transform_expression(node.to_expression(), false).map(ForStatementInit::from),
      });

      let test = test.as_ref().map(|test| self.transform_expression(test, true).unwrap());

      let update = update.as_ref().and_then(|update| self.transform_expression(update, false));

      let body =
        self.transform_statement(body).unwrap_or_else(|| self.ast.statement_empty(body.span()));

      Some(self.ast.statement_for(*span, init, test, update, body))
    } else {
      let init = init.as_ref().and_then(|init| match init {
        ForStatementInit::VariableDeclaration(node) => {
          self.transform_variable_declaration(node).map(Statement::VariableDeclaration)
        }
        node => self
          .transform_expression(node.to_expression(), false)
          .map(|inner| self.ast.statement_expression(inner.span(), inner)),
      });

      let test = test
        .as_ref()
        .and_then(|test| self.transform_expression(test, false))
        .map(|test| self.ast.statement_expression(test.span(), test));

      match (init, test) {
        (Some(init), test) => {
          let mut statements = self.ast.vec_with_capacity(2);
          statements.push(init);
          if let Some(test) = test {
            statements.push(test);
          }
          Some(self.ast.statement_block(*span, statements))
        }
        (None, Some(test)) => Some(test),
        (None, None) => None,
      }
    }
  }
}
