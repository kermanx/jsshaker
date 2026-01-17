use oxc::{
  allocator,
  ast::ast::{Argument, Expression},
  span::GetSpan,
};

use crate::{
  Analyzer,
  ast::{Arguments, AstKind2},
  entity::Entity,
  transformer::Transformer,
  value::ArgumentsValue,
};

impl<'a> Analyzer<'a> {
  pub fn exec_arguments(&mut self, node: &'a Arguments<'a>) -> ArgumentsValue<'a> {
    let mut elements = self.factory.vec();
    let mut rest = self.factory.vec();

    for element in node {
      let mut push_element = |element: Entity<'a>| {
        if rest.is_empty() {
          elements.push(element);
        } else {
          rest.push(element);
        }
      };
      match element {
        Argument::SpreadElement(node) => {
          let (els, r, d) = self.exec_spread_element(node);
          for el in els {
            push_element(self.factory.computed(el, d));
          }
          if let Some(r) = r {
            rest.push(self.factory.computed(r, d));
          }
        }
        _ => {
          let dep = AstKind2::Argument(element);
          let element = self.exec_expression(element.to_expression());
          push_element(self.factory.computed(element, dep));
        }
      }
    }

    ArgumentsValue { elements: elements.into_bump_slice(), rest: self.factory.try_union(rest) }
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_arguments_need_call(&self, node: &'a Arguments<'a>) -> Arguments<'a> {
    let mut arguments_rev = vec![];
    let mut preserve_args_num = false;
    for argument in node.into_iter().rev() {
      if let Some(argument) = self.transform_argument_need_call(argument, preserve_args_num) {
        arguments_rev.push(argument);
        preserve_args_num = true;
      }
    }
    allocator::Vec::from_iter_in(arguments_rev.into_iter().rev(), self.allocator)
  }

  fn transform_argument_need_call(
    &self,
    node: &'a Argument<'a>,
    preserve_args_num: bool,
  ) -> Option<Argument<'a>> {
    let included = self.is_included(AstKind2::Argument(node));
    let span = node.span();
    match node {
      Argument::SpreadElement(node) => self.transform_arguments_spread_element(node),
      _ => self
        .transform_expression(node.to_expression(), included)
        .or_else(|| preserve_args_num.then(|| self.build_unused_expression(span)))
        .map(Argument::from),
    }
  }

  pub fn transform_arguments_no_call(
    &self,
    node: &'a Arguments<'a>,
  ) -> Vec<Option<Expression<'a>>> {
    node.into_iter().map(|arg| self.transform_argument_no_call(arg)).collect()
  }

  fn transform_argument_no_call(&self, node: &'a Argument<'a>) -> Option<Expression<'a>> {
    match node {
      Argument::SpreadElement(node) => self.transform_expression(&node.argument, false),
      _ => self.transform_expression(node.to_expression(), false),
    }
  }
}
