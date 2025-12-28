use oxc::{
  allocator,
  ast::ast::{IdentifierReference, PropertyKey},
};

use crate::{analyzer::Analyzer, ast::AstKind2, entity::Entity, transformer::Transformer};

impl<'a> Analyzer<'a> {
  pub fn exec_identifier_reference_read(
    &mut self,
    node: &'a IdentifierReference<'a>,
  ) -> Entity<'a> {
    let reference = self.semantic().scoping().get_reference(node.reference_id());
    let symbol = reference.symbol_id();

    let dep = AstKind2::IdentifierReference(node);

    if let Some(symbol) = symbol {
      // Known symbol
      if let Some(value) = self.read_symbol(symbol) {
        value
      } else {
        // TDZ
        self.consume(dep);
        self.factory.unknown
      }
    } else if node.name == "arguments" {
      // The `arguments` object
      let arguments_consumed = self.consume_arguments();
      self.call_scope_mut().need_consume_arguments = !arguments_consumed;
      self.factory.unknown
    } else if let Some(global) = self.builtins.globals.get(node.name.as_str()) {
      // Known global
      *global
    } else {
      // Unknown global
      if self.config.unknown_global_side_effects {
        self.consume(dep);
        self.global_effect();
      }
      self.factory.unknown
    }
  }

  pub fn exec_identifier_reference_write(
    &mut self,
    node: &'a IdentifierReference<'a>,
    value: Entity<'a>,
  ) {
    let dep = AstKind2::IdentifierReference(node);
    let value = self.factory.computed(value, dep);

    let reference = self.semantic().scoping().get_reference(node.reference_id());
    assert!(reference.is_write());
    let symbol = reference.symbol_id();

    if let Some(symbol) = symbol {
      self.write_symbol(symbol, value);
    } else if self.builtins.globals.contains_key(node.name.as_str()) {
      self.add_diagnostic(
        "Should not write to builtin object, it may cause unexpected tree-shaking behavior",
      );
    } else {
      self.consume(dep);
      self.consume(value);
      self.global_effect();
    }
  }

  pub fn exec_identifier_reference_as_key(
    &mut self,
    node: &'a IdentifierReference<'a>,
  ) -> Entity<'a> {
    self.exec_mangable_static_string(AstKind2::IdentifierReference(node), node.name.as_str())
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_identifier_reference_read(
    &self,
    node: &'a IdentifierReference<'a>,
    need_val: bool,
  ) -> Option<allocator::Box<'a, IdentifierReference<'a>>> {
    self.transform_identifier_reference(node, need_val)
  }

  pub fn transform_identifier_reference_write(
    &self,
    node: &'a IdentifierReference<'a>,
  ) -> Option<allocator::Box<'a, IdentifierReference<'a>>> {
    self.transform_identifier_reference(node, false)
  }

  fn transform_identifier_reference(
    &self,
    node: &'a IdentifierReference<'a>,
    need_val: bool,
  ) -> Option<allocator::Box<'a, IdentifierReference<'a>>> {
    if need_val || self.is_deoptimized(AstKind2::IdentifierReference(node)) {
      let IdentifierReference { span, name, .. } = node;

      let reference = self.semantic.scoping().get_reference(node.reference_id());
      if let Some(symbol) = reference.symbol_id() {
        self.update_var_decl_state(symbol, false);
      }

      Some(self.ast.alloc_identifier_reference(*span, *name))
    } else {
      None
    }
  }

  pub fn transform_identifier_reference_as_key(
    &self,
    node: &'a IdentifierReference<'a>,
  ) -> PropertyKey<'a> {
    let IdentifierReference { span, name, .. } = node;
    self.ast.property_key_static_identifier(
      *span,
      self.transform_mangable_static_string(AstKind2::IdentifierReference(node), name),
    )
  }
}
