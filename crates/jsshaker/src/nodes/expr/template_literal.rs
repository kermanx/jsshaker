use std::mem;

use oxc::{
  allocator::FromIn,
  ast::ast::{Expression, TemplateElementValue, TemplateLiteral},
  span::{Atom, GetSpan, SPAN},
};

use crate::{
  analyzer::Analyzer, build_effect, entity::Entity, transformer::Transformer, utils::ast::AstKind2,
};

impl<'a> Analyzer<'a> {
  pub fn exec_template_literal(&mut self, node: &'a TemplateLiteral<'a>) -> Entity<'a> {
    let mut result = self.factory.unmangable_string(node.quasis[0].value.cooked.as_ref().unwrap());
    for (index, expression) in node.expressions.iter().enumerate() {
      let expression = self.exec_expression(expression);
      let quasi = self
        .factory
        .unmangable_string(node.quasis.get(index + 1).unwrap().value.cooked.as_ref().unwrap());
      result = self.op_add(result, expression);
      result = self.op_add(result, quasi);
    }
    result
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_template_literal(
    &self,
    node: &'a TemplateLiteral<'a>,
    need_val: bool,
  ) -> Option<Expression<'a>> {
    let TemplateLiteral { span, expressions, quasis } = node;
    if need_val {
      let mut quasis_iter = quasis.into_iter();
      let mut transformed_exprs = self.ast.vec();
      let mut transformed_quasis = vec![];
      let mut pending_effects = vec![];
      transformed_quasis
        .push(quasis_iter.next().unwrap().value.cooked.as_ref().unwrap().to_string());
      let exprs_len = expressions.len();
      for (index, expr) in expressions.into_iter().enumerate() {
        let is_last = index == exprs_len - 1;
        let expr_span = expr.span();
        let quasi_str = quasis_iter.next().unwrap().value.cooked.as_ref().unwrap().to_string();
        if let Some(literal) = self.get_folded_literal(AstKind2::Expression(expr)) {
          if let Some(effect) = self.transform_expression(expr, false) {
            pending_effects.push(Some(effect));
          }
          if !pending_effects.is_empty() && is_last {
            transformed_exprs.push(build_effect!(
              &self.ast,
              expr_span,
              mem::take(&mut pending_effects);
              literal.build_expr(self, SPAN, None)
            ));
            transformed_quasis.push(quasi_str);
          } else {
            let last_quasi = transformed_quasis.pop().unwrap();
            let expr_str = literal.to_string(self.allocator);
            transformed_quasis.push(format!("{}{}{}", last_quasi, expr_str, quasi_str));
          }
        } else {
          let expr = self.transform_expression(expr, true).unwrap();
          transformed_exprs.push(build_effect!(
            &self.ast,
            expr_span,
            mem::take(&mut pending_effects);
            expr
          ));
          transformed_quasis.push(quasi_str);
        }
      }
      if transformed_exprs.is_empty() {
        let s = transformed_quasis.pop().unwrap();
        Some(build_effect!(
          &self.ast,
          *span,
          pending_effects;
          self.ast.expression_string_literal(*span, self.ast.atom(&s), None)
        ))
      } else {
        assert!(pending_effects.is_empty());
        let mut quasis = self.ast.vec();
        let quasis_len = transformed_quasis.len();
        for (index, quasi) in transformed_quasis.into_iter().enumerate() {
          quasis.push(self.ast.template_element(
            *span,
            TemplateElementValue {
              // FIXME: escape
              raw: self.escape_template_element_value(&quasi).into(),
              cooked: Some(Atom::from_in(&quasi, self.allocator)),
            },
            index == quasis_len - 1,
          ));
        }
        Some(self.ast.expression_template_literal(*span, quasis, transformed_exprs))
      }
    } else {
      build_effect!(
        &self.ast,
        *span,
        expressions.into_iter().map(|x| self.transform_expression(x, false)).collect::<Vec<_>>()
      )
    }
  }
}
