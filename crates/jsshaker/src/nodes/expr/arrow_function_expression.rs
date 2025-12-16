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
  utils::CalleeNode,
  value::{ArgumentsValue, FunctionValue, cache::FnCacheTrackingData},
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
    func: &'a FunctionValue<'a>,
    call_dep: Dep<'a>,
    node: &'a ArrowFunctionExpression<'a>,
    variable_scopes: &'a [VariableScopeId],
    args: ArgumentsValue<'a>,
    consume: bool,
  ) -> (Entity<'a>, FnCacheTrackingData<'a>) {
    let runner = move |analyzer: &mut Analyzer<'a>| {
      analyzer.push_call_scope(
        func,
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
        runner(analyzer).0.consume(analyzer);
        analyzer.factory.never
      });
      (self.factory.unknown, FnCacheTrackingData::worst_case())
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
    let ArrowFunctionExpression { span, expression, r#async, params, body, .. } = node;

    if self.is_referred(AstKind2::ArrowFunctionExpression(node)) {
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
    } else if need_val {
      Some(
        self.ast_builder.expression_arrow_function(
          *span,
          true,
          false,
          NONE,
          self.ast_builder.formal_parameters(
            params.span,
            params.kind,
            self.ast_builder.vec(),
            NONE,
          ),
          NONE,
          self.ast_builder.function_body(
            body.span,
            self.ast_builder.vec(),
            self.ast_builder.vec1(
              self
                .ast_builder
                .statement_expression(body.span, self.build_unused_expression(body.span)),
            ),
          ),
        ),
      )
    } else {
      None
    }
  }
}
