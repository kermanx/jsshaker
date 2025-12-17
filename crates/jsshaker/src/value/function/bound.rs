use oxc::span::Span;

use crate::{
  Analyzer,
  entity::Entity,
  value::{ArgumentsValue, cache::FnCacheTrackingData, call::FnCallInfo},
};

#[derive(Debug)]
pub struct BoundFunction<'a> {
  pub span: Span,
  pub target: Entity<'a>,
  pub bound_this: Entity<'a>,
  pub bound_args: ArgumentsValue<'a>,
}

impl<'a> Analyzer<'a> {
  pub fn call_bound_function<const IS_CTOR: bool>(
    &mut self,
    bound_fn: &'a BoundFunction<'a>,
    info: FnCallInfo<'a>,
  ) -> (Entity<'a>, FnCacheTrackingData<'a>) {
    self.push_call_scope(info, false, false);

    let args = ArgumentsValue::from_concatenate(self, bound_fn.bound_args, info.args);
    let ret = bound_fn.target.call(
      self,
      info.call_dep,
      if IS_CTOR { info.this } else { bound_fn.bound_this },
      args,
    );
    self.return_value(ret, self.factory.no_dep);

    if info.consume {
      self.consume_return_values();
    }

    self.pop_call_scope()
  }
}
