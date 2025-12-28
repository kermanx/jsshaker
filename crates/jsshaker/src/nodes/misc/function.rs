use oxc::{
  allocator,
  ast::{
    NONE,
    ast::{Function, FunctionType},
  },
};

use crate::{
  analyzer::Analyzer,
  ast::{AstKind2, DeclarationKind},
  dep::DepAtom,
  entity::Entity,
  transformer::Transformer,
  utils::CalleeNode,
  value::{cache::FnCacheTrackingData, call::FnCallInfo},
};

impl<'a> Analyzer<'a> {
  pub fn exec_function(&mut self, node: &'a Function<'a>) -> Entity<'a> {
    if self.has_no_shake_notation(node.span) {
      return self.factory.computed_unknown(AstKind2::FunctionNoShake(node));
    }
    self.new_function(CalleeNode::Function(node)).into()
  }

  pub fn declare_function(
    &mut self,
    node: &'a Function<'a>,
    exporting: Option<DepAtom>,
  ) -> Entity<'a> {
    let entity = self.exec_function(node);

    if let Some(id) = node.id.as_ref() {
      self.declare_symbol(
        id.symbol_id(),
        AstKind2::BindingIdentifier(id),
        exporting,
        DeclarationKind::Function,
        Some(self.factory.computed(entity, AstKind2::BindingIdentifier(id))),
      );
    }

    entity
  }

  pub fn call_function(
    &mut self,
    node: &'a Function<'a>,
    info: FnCallInfo<'a>,
  ) -> (Entity<'a>, FnCacheTrackingData<'a>) {
    let runner = move |analyzer: &mut Analyzer<'a>| {
      analyzer.push_call_scope(info, node.r#async, node.generator);

      let factory = analyzer.factory;
      let variable_scope = analyzer.variable_scope_mut();
      variable_scope.this = Some(info.this);
      variable_scope.arguments =
        Some((info.args, factory.vec(/* later filled by formal parameters */)));

      let declare_in_body = node.r#type == FunctionType::FunctionExpression && node.id.is_some();
      if declare_in_body {
        let id = node.id.as_ref().unwrap();
        analyzer.declare_symbol(
          id.symbol_id(),
          AstKind2::BindingIdentifier(id),
          None,
          DeclarationKind::NamedFunctionInBody,
          Some(analyzer.factory.computed(info.func.into(), AstKind2::BindingIdentifier(id))),
        );
      }

      analyzer.exec_formal_parameters(&node.params, info.args, DeclarationKind::FunctionParameter);
      analyzer.exec_function_body(node.body.as_ref().unwrap());

      if info.consume {
        analyzer.consume_return_values();
      }

      analyzer.pop_call_scope()
    };

    if !info.consume && (node.r#async || node.generator) {
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
  pub fn transform_function(
    &self,
    node: &'a Function<'a>,
    need_val: bool,
  ) -> Option<allocator::Box<'a, Function<'a>>> {
    if self.is_referred(AstKind2::FunctionNoShake(node)) {
      return Some(self.ast.alloc(self.clone_node(node)));
    }

    let Function { r#type, span, id, generator, r#async, params, body, .. } = node;

    let need_id = id.as_ref().is_some_and(|id| self.is_referred(AstKind2::BindingIdentifier(id)));
    if self.is_referred(AstKind2::Function(node)) {
      let old_declaration_only = self.declaration_only.replace(false);

      let params = self.transform_formal_parameters(params);

      let body =
        body.as_ref().map(|body| self.transform_function_body(node.scope_id.get().unwrap(), body));

      if let Some(id) = id {
        let symbol = id.symbol_id.get().unwrap();
        self.update_var_decl_state(symbol, true);
      }

      self.declaration_only.set(old_declaration_only);

      Some(self.ast.alloc_function(
        *span,
        *r#type,
        if need_id { id.clone() } else { None },
        *generator,
        *r#async,
        false,
        NONE,
        NONE,
        params,
        NONE,
        body,
      ))
    } else if need_val || need_id {
      Some(self.ast.alloc_function(
        *span,
        *r#type,
        if need_id { id.clone() } else { None },
        *generator,
        *r#async,
        false,
        NONE,
        NONE,
        self.ast.formal_parameters(params.span, params.kind, self.ast.vec(), NONE),
        NONE,
        Some(self.ast.function_body(params.span, self.ast.vec(), self.ast.vec())),
      ))
    } else {
      None
    }
  }
}
