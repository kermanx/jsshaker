use crate::{
  analyzer::Analyzer, ast::AstKind2, build_effect, scope::CfScopeKind, transformer::Transformer,
};
use oxc::{
  ast::ast::{DoWhileStatement, NumberBase, Statement},
  span::GetSpan,
};

#[derive(Debug, Default, Clone)]
pub struct Data {
  need_test: bool,
  need_loop: bool,
}

impl<'a> Analyzer<'a> {
  pub fn exec_do_while_statement(&mut self, node: &'a DoWhileStatement<'a>) {
    self.push_cf_scope(CfScopeKind::LoopBreak, Some(false));

    // Execute the first round.
    self.push_cf_scope(CfScopeKind::LoopContinue, Some(false));
    self.exec_statement(&node.body);
    self.pop_cf_scope();

    if self.cf_scope().must_exited() {
      self.pop_cf_scope();
      return;
    }

    let data = self.load_data::<Data>(AstKind2::DoWhileStatement(node));
    data.need_test = true;
    let test = self.exec_expression(&node.test);

    // The rest is the same as while statement.
    if test.test_truthy() == Some(false) {
      self.pop_cf_scope();
      return;
    }
    test.consume(self);

    data.need_loop = true;

    self.exec_loop(move |analyzer| {
      analyzer.push_cf_scope(CfScopeKind::LoopContinue, None);

      analyzer.exec_statement(&node.body);
      analyzer.exec_expression(&node.test).consume(analyzer);

      analyzer.pop_cf_scope();
    });

    self.pop_cf_scope();
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_do_while_statement(
    &self,
    node: &'a DoWhileStatement<'a>,
  ) -> Option<Statement<'a>> {
    let data = self.get_data::<Data>(AstKind2::DoWhileStatement(node));

    let DoWhileStatement { span, test, body } = node;
    let body_span = body.span();
    let test_span = test.span();

    let body = self.transform_statement(body);

    // if !data.need_test {
    //   body
    // } else {
    // FIXME: Temporarily added `body.is_none()` because can't handle break/continue
    if !data.need_loop && body.is_none() {
      // match (body, test) {
      //   (Some(body), Some(test)) => {
      //     let mut statements = self.ast_builder.vec();
      //     statements.push(body);
      //     statements.push(self.ast_builder.statement_expression(*span, test));
      //     Some(self.ast_builder.statement_block(*span, statements))
      //   }
      //   (None, Some(test)) => Some(self.ast_builder.statement_expression(*span, test)),
      //   (Some(body), None) => Some(body),
      //   (None, None) => None,
      // }
      let test = data.need_test.then(|| self.transform_expression(test, false)).flatten();
      test.map(|test| self.ast_builder.statement_expression(*span, test))
    } else {
      Some(self.ast_builder.statement_do_while(
        *span,
        body.unwrap_or_else(|| self.ast_builder.statement_empty(body_span)),
        if !data.need_test {
          self.ast_builder.expression_numeric_literal(
            test.span(),
            0.0,
            Some("0".into()),
            NumberBase::Decimal,
          )
        } else if data.need_loop {
          self.transform_expression(test, true).unwrap()
        } else {
          build_effect!(
            self.ast_builder,
            test_span,
            self.transform_expression(test, false);
            self.build_unused_expression(test_span)
          )
        },
      ))
    }
    // }
  }
}
