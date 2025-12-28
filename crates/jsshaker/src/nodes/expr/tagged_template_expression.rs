use oxc::{
  ast::{
    NONE,
    ast::{Expression, TaggedTemplateExpression, TemplateLiteral},
  },
  span::GetSpan,
};

use crate::{
  analyzer::Analyzer, ast::AstKind2, build_effect, entity::Entity, transformer::Transformer,
};

impl<'a> Analyzer<'a> {
  pub fn exec_tagged_template_expression(
    &mut self,
    node: &'a TaggedTemplateExpression<'a>,
  ) -> Entity<'a> {
    let (_, tag, _, this) = match self.exec_callee(&node.tag) {
      Ok(v) => v,
      Err(v) => return v,
    };

    let mut arguments = self.factory.vec1(self.factory.unknown);

    for expr in &node.quasi.expressions {
      let value = self.exec_expression(expr);
      let dep = AstKind2::ExpressionInTaggedTemplate(expr);
      arguments.push(self.factory.computed(value, dep));
    }

    tag.call(
      self,
      AstKind2::TaggedTemplateExpression(node),
      this,
      self.factory.arguments(arguments.into_bump_slice(), None),
    )
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_tagged_template_expression(
    &self,
    node: &'a TaggedTemplateExpression<'a>,
    need_val: bool,
  ) -> Option<Expression<'a>> {
    let TaggedTemplateExpression { span, tag, quasi, .. } = node;

    let need_call = need_val || self.is_referred(AstKind2::TaggedTemplateExpression(node));

    let tag = self.transform_callee(tag, need_call).unwrap();

    if need_call {
      Some(self.ast.expression_tagged_template(
        *span,
        tag.unwrap(),
        NONE,
        self.transform_quasi(quasi),
      ))
    } else {
      build_effect!(
        &self.ast,
        *span,
        tag,
        quasi.expressions.iter().map(|x| self.transform_expression(x, false)).collect::<Vec<_>>()
      )
    }
  }

  fn transform_quasi(&self, node: &'a TemplateLiteral<'a>) -> TemplateLiteral<'a> {
    let TemplateLiteral { span, quasis, expressions } = node;

    let mut transformed_expressions = self.ast.vec();
    for expr in expressions {
      let expr_span = expr.span();
      let referred = self.is_referred(AstKind2::ExpressionInTaggedTemplate(expr));
      transformed_expressions.push(
        self
          .transform_expression(expr, referred)
          .unwrap_or_else(|| self.build_unused_expression(expr_span)),
      );
    }

    self.ast.template_literal(*span, self.clone_node(quasis), transformed_expressions)
  }
}
