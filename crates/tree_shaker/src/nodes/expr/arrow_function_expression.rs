use oxc::ast::{
  NONE,
  ast::{ArrowFunctionExpression, Expression},
};

use crate::{
  analyzer::Analyzer,
  ast::{AstKind2, DeclarationKind},
  dep::Dep,
  entity::Entity,
  scope::VariableScopeId,
  transformer::Transformer,
  utils::{CalleeInfo, CalleeNode},
};

impl<'a> Analyzer<'a> {
  pub fn exec_arrow_function_expression(
    &mut self,
    node: &'a ArrowFunctionExpression<'a>,
  ) -> Entity<'a> {
    self.new_function(CalleeNode::ArrowFunctionExpression(node)).into()
  }

  pub fn call_arrow_function_expression(
    &mut self,
    callee: CalleeInfo<'a>,
    call_dep: Dep<'a>,
    node: &'a ArrowFunctionExpression<'a>,
    variable_scopes: &'a [VariableScopeId],
    args: Entity<'a>,
    consume: bool,
  ) -> Entity<'a> {
    let runner = move |analyzer: &mut Analyzer<'a>| {
      analyzer.push_call_scope(
        callee,
        call_dep,
        variable_scopes.to_vec(),
        node.r#async,
        false,
        consume,
      );

      analyzer.exec_formal_parameters(&node.params, args, DeclarationKind::ArrowFunctionParameter);
      if node.expression {
        analyzer.exec_function_expression_body(&node.body);
      } else {
        analyzer.exec_function_body(&node.body);
      }

      if consume {
        analyzer.consume_return_values();
      }

      analyzer.pop_call_scope()
    };

    if !consume && node.r#async {
      // Too complex to analyze the control flow, thus run exhaustively
      self.exec_async_or_generator_fn(move |analyzer| {
        runner(analyzer).consume(analyzer);
      });
      self.factory.unknown
    } else {
      runner(self)
    }
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_arrow_function_expression(
    &self,
    node: &'a ArrowFunctionExpression<'a>,
    need_val: bool,
  ) -> Option<Expression<'a>> {
    if need_val || self.is_referred(AstKind2::ArrowFunctionExpression(node)) {
      let ArrowFunctionExpression { span, expression, r#async, params, body, .. } = node;

      let params = self.transform_formal_parameters(params);
      let body = if *expression {
        self.transform_function_expression_body(body)
      } else {
        self.transform_function_body(node.scope_id.get().unwrap(), body)
      };

      Some(self.ast_builder.expression_arrow_function(
        *span,
        *expression,
        *r#async,
        NONE,
        params,
        NONE,
        body,
      ))
    } else {
      None
    }
  }
}
