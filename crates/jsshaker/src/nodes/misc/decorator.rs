use oxc::{allocator, ast::ast::Decorator};

use crate::{
  analyzer::Analyzer, ast::AstKind2, entity::Entity, transformer::Transformer,
  value::ArgumentsValue,
};

impl<'a> Analyzer<'a> {
  /// Execute a single ES2025 Stage 3 decorator.
  ///
  /// Decorators receive two arguments: (value, context)
  /// - value: The decorated element (or undefined for fields)
  /// - context: A context object with metadata (simplified as unknown())
  ///
  /// Returns the decorator's return value, or the original value if undefined.
  ///
  /// NOTE: We call the decorator function to preserve side effects, but use
  /// a conservative approach for the return value to avoid deep recursion.
  pub fn exec_decorator(&mut self, decorator: &'a Decorator<'a>, value: Entity<'a>) -> Entity<'a> {
    // 1. Execute decorator expression to get decorator function
    let decorator_fn = self.exec_expression(&decorator.expression);

    // 2. Create dependency for this decorator
    let dep_id = AstKind2::Decorator(decorator);
    let dep = self.factory.dep(dep_id);
    self.cf_scope_mut().push_dep(dep);

    // 3. Build arguments: (value, context)
    // Use unknown as an over-approximation for the context object
    let mut elements = self.factory.vec();
    elements.push(self.factory.computed(value, dep));
    elements.push(self.factory.computed_unknown(dep));
    let args_value = ArgumentsValue { elements: elements.into_bump_slice(), rest: None };

    // 4. Call the decorator function (for side effects)
    // But don't use the result directly to avoid deep recursion
    let _result = decorator_fn.call(self, dep_id, self.factory.undefined, args_value);

    // 5. Conservative return: union of original value and unknown
    // This assumes the decorator could return undefined (keep original) or a new value
    // This avoids "exhaustive too deep" errors while preserving side effects
    self.factory.computed_unknown((value, dep))
  }

  /// Execute multiple ES2025 Stage 3 decorators in bottom-to-top order.
  ///
  /// Each decorator receives the result of the previous decorator (or the initial value).
  /// This matches the JavaScript spec for decorator execution order.
  pub fn exec_decorators(
    &mut self,
    decorators: &'a allocator::Vec<'a, Decorator<'a>>,
    initial_value: Entity<'a>,
  ) -> Entity<'a> {
    let mut current_value = initial_value;

    // Execute decorators bottom-to-top
    for decorator in decorators.iter().rev() {
      current_value = self.exec_decorator(decorator, current_value);
    }

    current_value
  }
}

impl<'a> Transformer<'a> {
  /// Transform decorators, only including those that are marked as included.
  ///
  /// This is used during the transformation phase to remove unused decorators.
  pub fn transform_decorators(
    &self,
    decorators: &'a allocator::Vec<'a, Decorator<'a>>,
  ) -> allocator::Vec<'a, Decorator<'a>> {
    let mut result = self.ast.vec();

    for decorator in decorators.iter() {
      // Only include decorator if it's marked as included
      if self.is_included(AstKind2::Decorator(decorator))
        && let Some(expr) = self.transform_expression(&decorator.expression, true)
      {
        result.push(self.ast.decorator(decorator.span, expr));
      }
    }

    result
  }
}
