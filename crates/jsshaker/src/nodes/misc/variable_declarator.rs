use oxc::{ast::ast::VariableDeclarator, span::GetSpan};

use crate::{
  analyzer::Analyzer,
  ast::{AstKind2, DeclarationKind},
  dep::DepAtom,
  entity::Entity,
  transformer::Transformer,
};

impl<'a> Analyzer<'a> {
  pub fn declare_variable_declarator(
    &mut self,
    node: &'a VariableDeclarator,
    exporting: Option<DepAtom>,
    kind: DeclarationKind,
  ) {
    self.declare_binding_pattern(&node.id, exporting, kind);
  }

  pub fn init_variable_declarator(
    &mut self,
    node: &'a VariableDeclarator,
    init: Option<Entity<'a>>,
  ) {
    let init = match init {
      Some(init) => {
        if node.init.is_some() {
          self.throw_builtin_error(
            "for-in/for-of loop variable declaration may not have an initializer",
          );
        }
        Some(init)
      }
      None => node.init.as_ref().map(|init| self.exec_expression(init)),
    }
    .map(|init| self.factory.computed(init, AstKind2::VariableDeclarator(node)));

    self.init_binding_pattern(&node.id, init);
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_variable_declarator(
    &self,
    node: &'a VariableDeclarator<'a>,
  ) -> Option<VariableDeclarator<'a>> {
    let VariableDeclarator { span, kind, id, init, .. } = node;

    let id_span = id.span();
    let id = self.transform_binding_pattern(id, false);

    let transformed_init = if self.declaration_only.get() {
      None
    } else {
      init.as_ref().and_then(|init| {
        self.transform_expression(init, self.is_referred(AstKind2::VariableDeclarator(node)))
      })
    };

    match (id, transformed_init) {
      (None, None) => None,
      (id, transformed_init) => Some(self.ast.variable_declarator(
        *span,
        *kind,
        id.unwrap_or_else(|| self.build_unused_binding_pattern(id_span)),
        if kind.is_const() {
          transformed_init
            .or_else(|| init.as_ref().map(|init| self.build_unused_expression(init.span())))
        } else {
          transformed_init
        },
        false,
      )),
    }
  }
}
