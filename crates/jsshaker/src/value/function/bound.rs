use oxc::span::Span;

use crate::{
  Analyzer,
  dep::Dep,
  entity::Entity,
  scope::VariableScopeId,
  utils::CalleeInfo,
  value::{ArgumentsValue, cache::FnCacheTrackingData},
};

#[derive(Debug)]
pub struct BoundFunction<'a> {
  pub span: Span,
  pub target: Entity<'a>,
  pub bound_this: Entity<'a>,
  pub bound_args: ArgumentsValue<'a>,
}

impl<'a> Analyzer<'a> {
  pub fn call_bound_function(
    &mut self,
    callee: CalleeInfo<'a>,
    call_dep: Dep<'a>,
    bound_fn: &'a BoundFunction<'a>,
    lexical_scope: Option<VariableScopeId>,
    ctor_this: Option<Entity<'a>>,
    args: ArgumentsValue<'a>,
    consume: bool,
  ) -> (Entity<'a>, FnCacheTrackingData<'a>) {
    self.push_call_scope(callee, call_dep, lexical_scope, false, false, consume);

    // self.exec_formal_parameters(&node.params, args, DeclarationKind::ArrowFunctionParameter);
    // if node.expression {
    //   self.exec_function_expression_body(&node.body);
    // } else {
    //   self.exec_function_body(&node.body);
    // }

    let args = ArgumentsValue::from_concatenate(self, bound_fn.bound_args, args);
    let ret = bound_fn.target.call(self, call_dep, ctor_this.unwrap_or(bound_fn.bound_this), args);
    self.return_value(ret, self.factory.no_dep);

    if consume {
      self.consume_return_values();
    }

    self.pop_call_scope()
  }
}
