use oxc::{
  ast::{
    NONE,
    ast::{CatchClause, CatchParameter},
  },
  span::GetSpan,
};

use crate::{analyzer::Analyzer, ast::DeclarationKind, entity::Entity, transformer::Transformer};

impl<'a> Analyzer<'a> {
  pub fn exec_catch_clause(&mut self, node: &'a CatchClause<'a>, value: Entity<'a>) {
    self.push_non_det_cf_scope();

    if let Some(param) = &node.param {
      self.declare_binding_pattern(&param.pattern, None, DeclarationKind::Caught);
      self.init_binding_pattern(&param.pattern, DeclarationKind::Caught, Some(value));
    }

    self.exec_block_statement(&node.body);

    self.pop_cf_scope();
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_catch_clause(&self, node: &'a CatchClause<'a>) -> CatchClause<'a> {
    let CatchClause { span, param, body, .. } = node;

    let param = param.as_ref().and_then(|param| {
      let CatchParameter { span, pattern, .. } = param;
      self
        .transform_binding_pattern(pattern, false)
        .map(|pattern| self.ast.catch_parameter(*span, pattern, NONE))
    });

    let body_span = body.span();
    let body = self.transform_block_statement(body);

    self.ast.catch_clause(
      *span,
      param,
      body.unwrap_or(self.ast.alloc_block_statement(body_span, self.ast.vec())),
    )
  }
}
