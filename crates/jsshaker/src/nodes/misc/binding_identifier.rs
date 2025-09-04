use oxc::ast::ast::BindingIdentifier;

use crate::{
  analyzer::Analyzer,
  ast::{AstKind2, DeclarationKind},
  entity::Entity,
  transformer::Transformer,
};

impl<'a> Analyzer<'a> {
  pub fn declare_binding_identifier(
    &mut self,
    node: &'a BindingIdentifier<'a>,
    exporting: bool,
    kind: DeclarationKind,
  ) {
    let symbol = node.symbol_id.get().unwrap();
    self.declare_symbol(symbol, AstKind2::BindingIdentifier(node), exporting, kind, None);
  }

  pub fn init_binding_identifier(
    &mut self,
    node: &'a BindingIdentifier<'a>,
    init: Option<Entity<'a>>,
  ) {
    let symbol = node.symbol_id.get().unwrap();
    self.init_symbol(symbol, init, AstKind2::BindingIdentifier(node));
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_binding_identifier(
    &self,
    node: &'a BindingIdentifier<'a>,
  ) -> Option<BindingIdentifier<'a>> {
    let symbol = node.symbol_id.get().unwrap();
    self.update_var_decl_state(symbol, true);

    let referred = self.is_referred(AstKind2::BindingIdentifier(node));
    referred.then(|| self.clone_node(node))
  }
}
