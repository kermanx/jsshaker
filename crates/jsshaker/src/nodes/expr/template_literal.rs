use oxc::{
  allocator::FromIn,
  ast::ast::{Expression, TemplateElementValue, TemplateLiteral},
  span::Atom,
};

use crate::{analyzer::Analyzer, build_effect, entity::Entity, transformer::Transformer};

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
      transformed_quasis
        .push(quasis_iter.next().unwrap().value.cooked.as_ref().unwrap().to_string());
      for expr in expressions {
        let expr = self.transform_expression(expr, true).unwrap();
        transformed_exprs.push(expr);

        let quasi_str = quasis_iter.next().unwrap().value.cooked.as_ref().unwrap().to_string();
        transformed_quasis.push(quasi_str);
      }
      if transformed_exprs.is_empty() {
        let s = transformed_quasis.pop().unwrap();
        Some(self.ast.expression_string_literal(*span, self.ast.atom(&s), None))
      } else {
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
