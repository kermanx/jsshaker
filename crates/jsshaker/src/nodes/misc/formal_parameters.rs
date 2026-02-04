use oxc::{
  ast::{
    NONE,
    ast::{FormalParameter, FormalParameters},
  },
  span::GetSpan,
};

use crate::{
  analyzer::Analyzer, ast::DeclarationKind, entity::Entity, transformer::Transformer,
  value::ArgumentsValue,
};

impl<'a> Analyzer<'a> {
  pub fn exec_formal_parameters(
    &mut self,
    node: &'a FormalParameters<'a>,
    args: ArgumentsValue<'a>,
    kind: DeclarationKind,
  ) {
    for param in &node.items {
      self.declare_binding_pattern(&param.pattern, None, kind);
    }

    for (param, &init) in node.items.iter().zip(args.elements) {
      self.exec_formal_parameter(param, init);
    }
    if node.items.len() > args.elements.len() {
      let value = args.rest.unwrap_or(self.factory.undefined);
      for param in &node.items[args.elements.len()..] {
        self.exec_formal_parameter(param, value);
      }
    }

    // In case of `function(x=arguments, y)`, `y` should also be consumed
    if self.call_scope_mut().need_consume_arguments {
      let arguments_consumed = self.consume_arguments();
      assert!(arguments_consumed);
    }

    if let Some(rest) = &node.rest {
      let arr = self.new_empty_array();
      if args.elements.len() > node.items.len() {
        for element in &args.elements[node.items.len()..] {
          arr.push_element(*element);
        }
      }
      if let Some(rest) = args.rest {
        arr.init_rest(rest);
      }

      self.declare_binding_rest_element(&rest.rest, None, kind);
      self.init_binding_rest_element(&rest.rest, arr.into());
    }
  }

  fn exec_formal_parameter(&mut self, node: &'a FormalParameter<'a>, arg: Entity<'a>) {
    let binding_val = if let Some(initializer) = &node.initializer {
      self.exec_with_default(initializer, arg)
    } else {
      arg
    };
    self.init_binding_pattern(&node.pattern, Some(binding_val));
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_formal_parameters(
    &self,
    node: &'a FormalParameters<'a>,
  ) -> FormalParameters<'a> {
    let FormalParameters { span, items, rest, kind } = node;

    let mut transformed_items = self.ast.vec();

    let mut counting_length = self.config.preserve_function_length;
    let mut used_length = 0;

    for (index, param) in items.iter().enumerate() {
      let FormalParameter { span, decorators, pattern, initializer, .. } = param;

      let pattern_span = pattern.span();
      let pattern = self.transform_binding_pattern(pattern, false);
      let initializer_span = initializer.as_ref().map(|init| init.span());
      let initializer = if let Some(initializer) = initializer {
        self.transform_with_default(initializer, pattern.is_some())
      } else {
        None
      };

      if initializer_span.is_some() {
        counting_length = false;
      }
      if counting_length || pattern.is_some() || initializer.is_some() {
        used_length = index + 1;
      }

      transformed_items.push(self.ast.formal_parameter(
        *span,
        self.transform_decorators(decorators),
        pattern.unwrap_or_else(|| self.build_unused_binding_pattern(pattern_span)),
        NONE,
        if counting_length
          && initializer.is_none()
          && let Some(span) = initializer_span
        {
          Some(self.build_unused_expression(span))
        } else {
          initializer
        },
        false,
        None,
        false,
        false,
      ));
    }

    let transformed_rest = match rest {
      Some(rest) => self
        .transform_binding_rest_element(&rest.rest, false)
        .map(|rest| self.ast.formal_parameter_rest(rest.span(), rest, NONE)),
      None => None,
    };

    transformed_items.truncate(used_length);

    self.ast.formal_parameters(*span, *kind, transformed_items, transformed_rest)
  }

  pub fn transform_uncalled_formal_parameters(
    &self,
    node: &'a FormalParameters<'a>,
  ) -> FormalParameters<'a> {
    let FormalParameters { span, items, kind, rest: _ } = node;

    if !self.config.preserve_function_length {
      return self.ast.formal_parameters(*span, *kind, self.ast.vec(), NONE);
    }

    let mut transformed_items = self.ast.vec();
    for param in items.iter() {
      let FormalParameter { span, decorators, initializer, .. } = param;

      if initializer.is_some() {
        break;
      }

      transformed_items.push(self.ast.formal_parameter(
        *span,
        self.transform_decorators(decorators),
        self.build_unused_binding_pattern(*span),
        NONE,
        NONE,
        false,
        None,
        false,
        false,
      ));
    }

    self.ast.formal_parameters(*span, *kind, transformed_items, NONE)
  }
}
