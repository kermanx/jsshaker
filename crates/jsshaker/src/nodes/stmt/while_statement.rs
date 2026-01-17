use oxc::{
  ast::ast::{Statement, WhileStatement},
  span::GetSpan,
};

use crate::{analyzer::Analyzer, ast::AstKind2, scope::CfScopeKind, transformer::Transformer};

impl<'a> Analyzer<'a> {
  pub fn exec_while_statement(&mut self, node: &'a WhileStatement<'a>) {
    // This may be non_det. However, we can't know it until we execute the test.
    // And there should be no same level break/continue statement in test.
    // `a: while(() => { break a }) { }` is illegal.
    let test = self.exec_expression(&node.test);

    if test.test_truthy() == Some(false) {
      return;
    }

    let dep = self.dep((AstKind2::WhileStatement(node), test));

    self.push_cf_scope_with_deps(CfScopeKind::LoopBreak, self.factory.vec1(dep), false);
    self.exec_loop(move |analyzer| {
      analyzer.push_cf_scope(CfScopeKind::LoopContinue, true);

      analyzer.exec_statement(&node.body);
      analyzer.exec_expression(&node.test).consume(analyzer);

      analyzer.pop_cf_scope();

      let test = analyzer.exec_expression(&node.test);
      let test = analyzer.dep(test);
      analyzer.cf_scope_mut().push_dep(test);
    });
    self.pop_cf_scope();
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_while_statement(&self, node: &'a WhileStatement<'a>) -> Option<Statement<'a>> {
    let WhileStatement { span, test, body } = node;
    let body_span = body.span();

    let need_loop = self.is_included(AstKind2::WhileStatement(node));
    let test = self.transform_expression(test, need_loop);
    let body = if need_loop { self.transform_statement(body) } else { None };

    match (test, body) {
      (Some(test), body) => Some(self.ast.statement_while(
        *span,
        test,
        body.unwrap_or_else(|| self.ast.statement_empty(body_span)),
      )),
      (None, Some(_)) => unreachable!(),
      (None, None) => None,
    }
  }
}
