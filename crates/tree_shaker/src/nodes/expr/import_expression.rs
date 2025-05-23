use oxc::ast::ast::{Expression, ImportExpression};

use crate::{
  analyzer::Analyzer, build_effect, entity::Entity, transformer::Transformer, utils::ast::AstKind2,
  value::LiteralValue,
};

impl<'a> Analyzer<'a> {
  pub fn exec_import_expression(&mut self, node: &'a ImportExpression<'a>) -> Entity<'a> {
    let specifier = self.exec_expression(&node.source).get_to_string(self);
    let options = node.options.as_ref().map(|option| self.exec_expression(option));
    let dep = self.dep((AstKind2::ImportExpression(node), specifier, options));

    if let Some(LiteralValue::String(specifier, _m)) = specifier.get_literal(self) {
      if let Some(module_id) = self.resolve_and_import_module(specifier) {
        return self.factory.computed_unknown((module_id, dep));
      }
    }

    self.factory.computed_unknown(dep)
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_import_expression(
    &self,
    node: &'a ImportExpression<'a>,
    need_val: bool,
  ) -> Option<Expression<'a>> {
    let ImportExpression { span, source, options, phase } = node;

    let need_import = need_val || self.is_referred(AstKind2::ImportExpression(node));

    let source = self.transform_expression(source, need_import);

    if need_import {
      Some(self.ast_builder.expression_import(
        *span,
        source.unwrap(),
        options.as_ref().map(|option| self.transform_expression(option, true).unwrap()),
        *phase,
      ))
    } else {
      build_effect!(
        &self.ast_builder,
        *span,
        source,
        options.as_ref().map(|option| self.transform_expression(option, false)),
      )
    }
  }
}
