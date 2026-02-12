use oxc::{
  ast::ast::{ForOfStatement, Statement},
  span::GetSpan,
};

use crate::{
  analyzer::Analyzer, ast::AstKind2, scope::CfScopeKind, transformer::Transformer,
};

impl<'a> Analyzer<'a> {
  pub fn exec_for_of_statement(&mut self, node: &'a ForOfStatement<'a>) {
    let right = self.exec_expression(&node.right);
    let right = if node.r#await {
      right.include(self);
      self.include_atom(AstKind2::ForOfStatement(node));
      self.factory.unknown
    } else {
      right
    };

    let (elements, mut rest, dep, array_ids) = right.iterate(self, AstKind2::ForOfStatement(node));

    let original_versions =
      array_ids.iter().map(|array_id| array_id.as_ref().version.get()).collect::<Vec<_>>();

    self.push_cf_scope_with_deps(CfScopeKind::LoopBreak, self.factory.vec1(dep), false);
    for element in elements {
      self.push_cf_scope_with_deps(
        CfScopeKind::LoopContinue,
        self.factory.vec1(element.get_shallow_dep(self)),
        false,
      );

      self.push_variable_scope();
      self.declare_for_statement_left(&node.left);
      self.init_for_statement_left(&node.left, element);

      self.exec_statement(&node.body);

      self.pop_variable_scope();
      self.pop_cf_scope();

      for (array_id, original_version) in array_ids.iter().zip(&original_versions) {
        let array = array_id.as_ref();
        if array.version.get() != *original_version {
          self.include(array);
          rest = Some(self.factory.unknown);
          break;
        }
      }

      if self.cf_scope().must_exited() {
        break;
      }
    }
    if !self.cf_scope().must_exited()
      && let Some(rest) = rest
    {
      let cf_dep = rest.get_shallow_dep(self);
      self.exec_loop(move |analyzer| {
        analyzer.push_cf_scope_with_deps(
          CfScopeKind::LoopContinue,
          analyzer.factory.vec1(cf_dep),
          true,
        );

        analyzer.declare_for_statement_left(&node.left);
        analyzer.init_for_statement_left(&node.left, rest);

        analyzer.exec_statement(&node.body);

        analyzer.pop_cf_scope();
      });
    }
    self.pop_cf_scope();
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_for_of_statement(&self, node: &'a ForOfStatement<'a>) -> Option<Statement<'a>> {
    let ForOfStatement { span, r#await, left, right, body, .. } = node;

    let need_loop = self.is_included(AstKind2::ForOfStatement(node));

    let left_span = left.span();
    let body_span = body.span();

    let left = if need_loop { self.transform_for_statement_left(left) } else { None };
    let body = if need_loop { self.transform_statement(body) } else { None };

    if left.is_none() && body.is_none() {
      return if self.is_included(AstKind2::ForOfStatement(node)) {
        let right_span = right.span();
        let right = self.transform_expression(right, true).unwrap();
        Some(if *r#await {
          self.ast.statement_for_of(
            *span,
            true,
            self.build_unused_for_statement_left(left_span),
            right,
            self.ast.statement_empty(body_span),
          )
        } else {
          self.ast.statement_expression(
            *span,
            self.ast.expression_array(
              *span,
              self.ast.vec1(self.ast.array_expression_element_spread_element(right_span, right)),
            ),
          )
        })
      } else {
        self
          .transform_expression(right, false)
          .map(|expr| self.ast.statement_expression(*span, expr))
      };
    }

    let right = self.transform_expression(right, true).unwrap();

    Some(self.ast.statement_for_of(
      *span,
      *r#await,
      left.unwrap_or_else(|| self.build_unused_for_statement_left(left_span)),
      right,
      body.unwrap_or_else(|| self.ast.statement_empty(body_span)),
    ))
  }
}
