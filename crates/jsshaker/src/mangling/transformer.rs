use oxc::{
  allocator::CloneIn,
  ast::ast::{IdentifierName, MethodDefinition, StringLiteral},
  span::Atom,
};
use oxc_ast_visit::{VisitMut, walk_mut};

use crate::{dep::DepAtom, transformer::Transformer, utils::ast::AstKind2};

pub struct ManglerTransformer<'a>(pub Transformer<'a>);

impl<'a> ManglerTransformer<'a> {
  pub fn resolve_node(&self, key: impl Into<DepAtom>, original: &'a str) -> Atom<'a> {
    let mut mangler = self.0.mangler.borrow_mut();
    mangler.resolve_node(key).unwrap_or(original).into()
  }
}

impl<'a> VisitMut<'a> for ManglerTransformer<'a> {
  fn visit_identifier_name(&mut self, node: &mut IdentifierName<'a>) {
    node.name = self.resolve_node(AstKind2::IdentifierName(node), node.name.as_str());
    walk_mut::walk_identifier_name(self, node);
  }

  fn visit_string_literal(&mut self, node: &mut StringLiteral<'a>) {
    node.value = self.resolve_node(AstKind2::StringLiteral(node), node.value.as_str());
    walk_mut::walk_string_literal(self, node);
  }

  fn visit_method_definition(&mut self, node: &mut MethodDefinition<'a>) {
    if node.kind.is_constructor() {
      let old_key = node.key.clone_in(self.0.allocator);
      walk_mut::walk_method_definition(self, node);
      node.key = old_key;
    } else {
      walk_mut::walk_method_definition(self, node);
    }
  }
}
