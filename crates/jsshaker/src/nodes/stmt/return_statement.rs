use oxc::ast::ast::{ReturnStatement, Statement};

use crate::{analyzer::Analyzer, ast::AstKind2, transformer::Transformer};

impl<'a> Analyzer<'a> {
  pub fn exec_return_statement(&mut self, node: &'a ReturnStatement) {
    let value =
      node.argument.as_ref().map_or(self.factory.undefined, |expr| self.exec_expression(expr));
    self.return_value(value, AstKind2::ReturnStatement(node));
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_return_statement(&self, node: &'a ReturnStatement<'a>) -> Option<Statement<'a>> {
    let need_val = self.is_deoptimized(AstKind2::ReturnStatement(node));

    let ReturnStatement { span, argument } = node;

    Some(self.ast.statement_return(
      *span,
      argument.as_ref().and_then(|arg| self.transform_expression(arg, need_val)),
    ))
  }
}
