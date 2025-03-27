use oxc::ast::ast::Declaration;

use crate::{analyzer::Analyzer, transformer::Transformer};

impl<'a> Analyzer<'a> {
  pub fn declare_declaration(&mut self, node: &'a Declaration<'a>, exporting: bool) {
    match node {
      Declaration::VariableDeclaration(node) => {
        self.declare_variable_declaration(node, exporting);
      }
      Declaration::FunctionDeclaration(node) => {
        self.declare_function(node, exporting);
      }
      Declaration::ClassDeclaration(node) => {
        self.declare_class(node, exporting);
      }
      _ => unreachable!(),
    }
  }

  pub fn init_declaration(&mut self, node: &'a Declaration<'a>) {
    match node {
      Declaration::VariableDeclaration(node) => {
        self.init_variable_declaration(node, None);
      }
      Declaration::FunctionDeclaration(_node) => {
        // Nothing to do
      }
      Declaration::ClassDeclaration(node) => {
        self.init_class(node);
      }
      _ => unreachable!(),
    }
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_declaration(&self, node: &'a Declaration<'a>) -> Option<Declaration<'a>> {
    match node {
      Declaration::VariableDeclaration(node) => {
        self.transform_variable_declaration(node).map(Declaration::VariableDeclaration)
      }
      Declaration::FunctionDeclaration(node) => {
        self.transform_function(node, false).map(Declaration::FunctionDeclaration)
      }
      Declaration::ClassDeclaration(node) => {
        self.transform_class(node, false).map(Declaration::ClassDeclaration)
      }
      _ => unreachable!(),
    }
  }
}
