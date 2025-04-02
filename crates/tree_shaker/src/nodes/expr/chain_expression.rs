use oxc::ast::{
  ast::{ChainElement, ChainExpression, Expression},
  match_member_expression,
};

use crate::{analyzer::Analyzer, entity::Entity, transformer::Transformer};

impl<'a> Analyzer<'a> {
  pub fn exec_chain_expression(&mut self, node: &'a ChainExpression<'a>) -> Entity<'a> {
    match &node.expression {
      ChainElement::CallExpression(node) => {
        let result = self.exec_call_expression_in_chain(node);
        match result {
          Ok((scope_count, value, undefined)) => {
            let exec_dep = self.pop_multiple_cf_scopes(scope_count);
            self.factory.optional_union(self.factory.optional_computed(value, exec_dep), undefined)
          }
          Err(value) => value,
        }
      }
      node => {
        let result = self.exec_member_expression_read_in_chain(node.to_member_expression(), false);
        match result {
          Ok((scope_count, value, undefined, _)) => {
            let exec_dep = self.pop_multiple_cf_scopes(scope_count);
            self.factory.optional_union(self.factory.optional_computed(value, exec_dep), undefined)
          }
          Err(value) => value,
        }
      }
    }
  }

  pub fn exec_expression_in_chain(
    &mut self,
    node: &'a Expression<'a>,
  ) -> Result<(usize, Entity<'a>, Option<Entity<'a>>), Entity<'a>> {
    match node {
      match_member_expression!(Expression) => self
        .exec_member_expression_read_in_chain(node.to_member_expression(), false)
        .map(|(scope_count, value, undefined, _)| (scope_count, value, undefined)),
      Expression::CallExpression(node) => self.exec_call_expression_in_chain(node),
      _ => Ok((0, self.exec_expression(node), None)),
    }
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_chain_expression(
    &self,
    node: &'a ChainExpression<'a>,
    need_val: bool,
  ) -> Option<Expression<'a>> {
    let ChainExpression { expression, .. } = node;

    match expression {
      ChainElement::CallExpression(node) => self.transform_call_expression_in_chain(node, need_val),
      node => self.transform_member_expression_read_in_chain(node.to_member_expression(), need_val),
    }
    .unwrap_or_else(|v| v)
  }

  pub fn transform_expression_in_chain(
    &self,
    node: &'a Expression<'a>,
    need_val: bool,
  ) -> Result<Option<Expression<'a>>, Option<Expression<'a>>> {
    match node {
      match_member_expression!(Expression) => {
        self.transform_member_expression_read_in_chain(node.to_member_expression(), need_val)
      }
      Expression::CallExpression(node) => self.transform_call_expression_in_chain(node, need_val),
      _ => Ok(self.transform_expression(node, need_val)),
    }
  }
}
