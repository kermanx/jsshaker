use oxc::{
  allocator,
  ast::ast::{VariableDeclaration, VariableDeclarationKind},
};

use crate::{
  analyzer::Analyzer, ast::DeclarationKind, dep::DepAtom, entity::Entity, transformer::Transformer,
};

impl<'a> Analyzer<'a> {
  pub fn declare_variable_declaration(
    &mut self,
    node: &'a VariableDeclaration<'a>,
    exporting: Option<DepAtom>,
  ) {
    let kind = match &node.kind {
      VariableDeclarationKind::Var => DeclarationKind::Var,
      VariableDeclarationKind::Let => DeclarationKind::Let,
      VariableDeclarationKind::Const => DeclarationKind::Const,
      _ => unimplemented!("using statement"),
    };

    for declarator in &node.declarations {
      self.declare_variable_declarator(declarator, exporting, kind);
    }
  }

  pub fn init_variable_declaration(
    &mut self,
    node: &'a VariableDeclaration<'a>,
    init: Option<Entity<'a>>,
  ) {
    if init.is_some() {
      assert_eq!(node.declarations.len(), 1);
    }

    for declarator in &node.declarations {
      self.init_variable_declarator(declarator, init);
    }
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_variable_declaration(
    &self,
    node: &'a VariableDeclaration<'a>,
  ) -> Option<allocator::Box<'a, VariableDeclaration<'a>>> {
    let VariableDeclaration { span, kind, declarations, .. } = node;
    let mut transformed_decls = self.ast_builder.vec();
    for declarator in declarations {
      let declarator = self.transform_variable_declarator(declarator);
      if let Some(declarator) = declarator {
        transformed_decls.push(declarator);
      }
    }
    if transformed_decls.is_empty() {
      None
    } else {
      Some(self.ast_builder.alloc_variable_declaration(*span, *kind, transformed_decls, false))
    }
  }
}
