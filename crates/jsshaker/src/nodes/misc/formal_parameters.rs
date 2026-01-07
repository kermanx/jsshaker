use oxc::{
  ast::{
    NONE,
    ast::{BindingPatternKind, FormalParameter, FormalParameters},
  },
  span::{GetSpan, SPAN},
};

use crate::{
  analyzer::Analyzer, ast::DeclarationKind, transformer::Transformer, value::ArgumentsValue,
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

    for (param, init) in node.items.iter().zip(args.elements) {
      self.init_binding_pattern(&param.pattern, Some(*init));
    }
    if node.items.len() > args.elements.len() {
      let value = args.rest.unwrap_or(self.factory.undefined);
      for param in &node.items[args.elements.len()..] {
        self.init_binding_pattern(&param.pattern, Some(value));
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

      self.declare_binding_rest_element(rest, None, kind);
      self.init_binding_rest_element(rest, arr.into());
    }
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
      let FormalParameter { span, decorators, pattern, .. } = param;

      let pattern_was_assignment = matches!(pattern.kind, BindingPatternKind::AssignmentPattern(_));
      let pattern = if let Some(pattern) = self.transform_binding_pattern(pattern, false) {
        used_length = index + 1;
        pattern
      } else {
        self.build_unused_binding_pattern(*span)
      };
      let pattern_is_assignment = matches!(pattern.kind, BindingPatternKind::AssignmentPattern(_));

      transformed_items.push(self.ast.formal_parameter(
        *span,
        self.clone_node(decorators),
        if counting_length && pattern_was_assignment && !pattern_is_assignment {
          self.ast.binding_pattern(
            self.ast.binding_pattern_kind_assignment_pattern(
              pattern.span(),
              pattern,
              self.build_unused_expression(SPAN),
            ),
            NONE,
            false,
          )
        } else {
          pattern
        },
        None,
        false,
        false,
      ));

      if pattern_was_assignment {
        counting_length = false;
      }
      if counting_length {
        used_length = index + 1;
      }
    }

    let transformed_rest = match rest {
      Some(rest) => self.transform_binding_rest_element(rest, false),
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
      let FormalParameter { span, decorators, pattern, .. } = param;

      if matches!(pattern.kind, BindingPatternKind::AssignmentPattern(_)) {
        break;
      }

      transformed_items.push(self.ast.formal_parameter(
        *span,
        self.clone_node(decorators),
        self.build_unused_binding_pattern(*span),
        None,
        false,
        false,
      ));
    }

    self.ast.formal_parameters(*span, *kind, transformed_items, NONE)
  }
}
