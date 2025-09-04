use oxc::ast::ast::Expression;
use oxc_syntax::operator::LogicalOperator;

use crate::{analyzer::Analyzer, entity::Entity, transformer::Transformer, utils::ast::AstKind2};

impl<'a> Analyzer<'a> {
  pub fn exec_with_default(
    &mut self,
    default: &'a Expression<'a>,
    original: Entity<'a>,
  ) -> Entity<'a> {
    let (maybe_original, maybe_fallback) = match original.test_is_undefined() {
      Some(true) => (false, true),
      Some(false) => (true, false),
      None => (true, true),
    };

    let forward_original = |analyzer: &mut Analyzer<'a>| {
      analyzer.forward_logical_left_val(
        AstKind2::WithDefault(default),
        original,
        maybe_original,
        maybe_fallback,
      )
    };
    let exec_fallback = |analyzer: &mut Analyzer<'a>| {
      let conditional_dep = analyzer.push_logical_right_cf_scope(
        AstKind2::WithDefault(default),
        original,
        maybe_original,
        maybe_fallback,
      );

      let val = analyzer.factory.computed(analyzer.exec_expression(default), conditional_dep);

      analyzer.pop_cf_scope();

      val
    };

    let result = match (maybe_original, maybe_fallback) {
      (true, false) => forward_original(self),
      (false, true) => exec_fallback(self),
      (true, true) => {
        let fallback = exec_fallback(self);
        let original = forward_original(self);
        self.factory.logical_result(fallback, original, LogicalOperator::Coalesce)
      }
      (false, false) => unreachable!(),
    };

    self.factory.computed(result, original.get_shallow_dep(self))
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_with_default(
    &self,
    default: &'a Expression<'a>,
    need_val: bool,
  ) -> Option<Expression<'a>> {
    let (_, _, maybe_fallback) = self.get_conditional_result(AstKind2::WithDefault(default));
    if maybe_fallback { self.transform_expression(default, need_val) } else { None }
  }
}
