use oxc::{
  ast::{
    NONE,
    ast::{CatchClause, CatchParameter},
  },
  span::GetSpan,
};

use crate::{
  analyzer::Analyzer, ast::DeclarationKind, dep::Dep, entity::Entity, scope::CfScopeKind,
  transformer::Transformer,
};

impl<'a> Analyzer<'a> {
  pub fn exec_catch_clause(
    &mut self,
    node: &'a CatchClause<'a>,
    value: Entity<'a>,
    exit_dep: Option<Dep<'a>>,
  ) {
    // The catch clause only runs when the try block throws, so its effects depend
    // on the control flow that leads to a throw (`exit_dep`).
    if let Some(exit_dep) = exit_dep {
      self.push_cf_scope_with_deps(CfScopeKind::NonDet, self.factory.vec1(exit_dep), true);
    } else {
      self.push_non_det_cf_scope();
    }

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
