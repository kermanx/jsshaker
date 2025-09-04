use oxc::ast::ast::{
  ExportDefaultDeclaration, ExportDefaultDeclarationKind, ExportNamedDeclaration,
  ImportDeclaration, ImportDeclarationSpecifier, ImportDefaultSpecifier, ImportNamespaceSpecifier,
  ImportSpecifier, ModuleDeclaration, ModuleExportName, PropertyKind,
};

use crate::{
  Analyzer, ast::DeclarationKind, module::NamedExport, transformer::Transformer,
  utils::ast::AstKind2, value::ObjectPrototype,
};

impl<'a> Analyzer<'a> {
  pub fn declare_module_declaration(&mut self, node: &'a ModuleDeclaration<'a>) {
    match node {
      ModuleDeclaration::ImportDeclaration(node) => {
        if let Some(specifiers) = &node.specifiers {
          for specifier in specifiers {
            self.declare_binding_identifier(specifier.local(), false, DeclarationKind::Import);
          }
        }
      }
      ModuleDeclaration::ExportNamedDeclaration(node) => {
        if let Some(source) = &node.source {
          if let Some(known) = self.builtins.get_known_module(&source.value) {
            for specifier in &node.specifiers {
              let value = known.namespace.get_property(
                self,
                AstKind2::ExportSpecifier(specifier),
                self.factory.string(specifier.local.name().as_str()),
              );
              self
                .module_info_mut()
                .named_exports
                .insert(specifier.exported.name(), NamedExport::Value(value));
            }
          } else if let Some(module) = self.resolve_and_import_module(&source.value) {
            for specifier in &node.specifiers {
              let dep = self.dep(AstKind2::ExportSpecifier(specifier));
              self.module_info_mut().named_exports.insert(
                specifier.exported.name(),
                NamedExport::ReExport(module, specifier.local.name(), dep),
              );
            }
          } else {
            for specifier in &node.specifiers {
              let value = self.factory.computed_unknown(AstKind2::ExportSpecifier(specifier));
              self
                .module_info_mut()
                .named_exports
                .insert(specifier.exported.name(), NamedExport::Value(value));
            }
          }
          return;
        }
        if let Some(declaration) = &node.declaration {
          self.declare_declaration(declaration, true);
        }
        for specifier in &node.specifiers {
          match &specifier.local {
            ModuleExportName::IdentifierReference(node) => {
              let dep = self.dep(AstKind2::ExportSpecifier(specifier));
              let reference = self.semantic().scoping().get_reference(node.reference_id());
              if let Some(symbol) = reference.symbol_id() {
                let scope = self.scoping.variable.current_id();
                self
                  .module_info_mut()
                  .named_exports
                  .insert(specifier.exported.name(), NamedExport::Variable(scope, symbol, dep));
              } else {
                self.consume(dep);
              }
            }
            _ => unreachable!(),
          }
        }
      }
      ModuleDeclaration::ExportDefaultDeclaration(node) => {
        match &node.declaration {
          ExportDefaultDeclarationKind::FunctionDeclaration(node) => {
            if node.id.is_none() {
              // Patch `export default function(){}`
              return;
            }
            // Pass `exporting` as `false` because it is actually used as an expression
            self.declare_function(node, false);
          }
          ExportDefaultDeclarationKind::ClassDeclaration(node) => {
            if node.id.is_none() {
              // Patch `export default class{}`
              return;
            }
            // Pass `exporting` as `false` because it is actually used as an expression
            self.declare_class(node, false);
          }
          _expr => {}
        };
      }
      ModuleDeclaration::ExportAllDeclaration(_node) => {
        // Nothing to do
      }
      _ => unreachable!(),
    }
  }

  pub fn init_import_declaration(&mut self, node: &'a ImportDeclaration<'a>) {
    if let Some(specifiers) = &node.specifiers {
      let name = node.source.value.as_str();
      let known = self.builtins.get_known_module(name);
      let resolved = if known.is_none() { self.resolve_and_import_module(name) } else { None };

      if let Some(resolved) = resolved {
        if self.module_stack.contains(&resolved) {
          // Circular dependency
          let module = self.current_module();
          let scope = self.scoping.variable.current_id();
          self.modules.modules[resolved].blocked_imports.push((module, scope, node));
          return;
        }
      }

      for specifier in specifiers {
        let value = if let Some(known) = known {
          match specifier {
            ImportDeclarationSpecifier::ImportDefaultSpecifier(_node) => known.default,
            ImportDeclarationSpecifier::ImportNamespaceSpecifier(_node) => known.namespace,
            ImportDeclarationSpecifier::ImportSpecifier(node) => {
              let key = self.factory.string(node.imported.name().as_str());
              known.namespace.get_property(self, self.factory.no_dep, key)
            }
          }
        } else if let Some(resolved) = resolved {
          let module_info = &self.modules.modules[resolved];
          match specifier {
            ImportDeclarationSpecifier::ImportDefaultSpecifier(_node) => {
              module_info.default_export.unwrap_or(self.factory.unknown)
            }
            ImportDeclarationSpecifier::ImportNamespaceSpecifier(_node) => {
              // FIXME: This is not accurate
              let named_exports = module_info.named_exports.clone();
              let object = self
                .new_empty_object(ObjectPrototype::Builtin(&self.builtins.prototypes.null), None);
              for (key, named_export) in named_exports {
                let value = self.get_named_export_value(named_export);
                object.init_property(
                  self,
                  PropertyKind::Init,
                  self.factory.string(key.as_str()),
                  value,
                  true,
                );
              }
              object.into()
            }
            ImportDeclarationSpecifier::ImportSpecifier(node) => {
              if let Some(named_export) =
                module_info.named_exports.get(&node.imported.name()).copied()
              {
                self.get_named_export_value(named_export)
              } else {
                self.factory.unknown
              }
            }
          }
        } else {
          self.builtins.factory.unknown
        };
        self.init_binding_identifier(specifier.local(), Some(value));
      }
    }
  }

  pub fn init_module_declaration(&mut self, node0: &'a ModuleDeclaration<'a>) {
    match node0 {
      ModuleDeclaration::ImportDeclaration(_node) => {
        // Hoisted
      }
      ModuleDeclaration::ExportNamedDeclaration(node) => {
        if node.source.is_some() {
          // Re-exports. Nothing to do.
          return;
        }
        if let Some(declaration) = &node.declaration {
          self.init_declaration(declaration);
        }
      }
      ModuleDeclaration::ExportDefaultDeclaration(node) => {
        let value = match &node.declaration {
          ExportDefaultDeclarationKind::FunctionDeclaration(node) => self.exec_function(node),
          ExportDefaultDeclarationKind::ClassDeclaration(node) => {
            if node.id.is_none() {
              // Patch `export default class{}`
              self.exec_class(node)
            } else {
              self.init_class(node)
            }
          }
          node => self.exec_expression(node.to_expression()),
        };
        if self.module_info_mut().default_export.is_some() {
          self.add_diagnostic("Duplicate default export");
        }
        self.module_info_mut().default_export = Some(value);
      }
      ModuleDeclaration::ExportAllDeclaration(_node) => {
        if self.module_stack.len() > 1 {
          todo!("ExportAllDeclaration");
        }
      }
      _ => unreachable!(),
    }
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_module_declaration(
    &self,
    node: &'a ModuleDeclaration<'a>,
  ) -> Option<ModuleDeclaration<'a>> {
    match node {
      ModuleDeclaration::ImportDeclaration(node) => {
        let ImportDeclaration { span, specifiers, source, with_clause, import_kind, phase } =
          node.as_ref();
        if let Some(specifiers) = specifiers {
          let mut transformed_specifiers = self.ast_builder.vec();
          for specifier in specifiers {
            let specifier = match specifier {
              ImportDeclarationSpecifier::ImportSpecifier(node) => {
                let ImportSpecifier { span, local, imported, import_kind } = node.as_ref();
                self.transform_binding_identifier(local).map(|local| {
                  self.ast_builder.import_declaration_specifier_import_specifier(
                    *span,
                    imported.clone(),
                    local,
                    *import_kind,
                  )
                })
              }
              ImportDeclarationSpecifier::ImportDefaultSpecifier(node) => {
                let ImportDefaultSpecifier { span, local } = node.as_ref();
                self.transform_binding_identifier(local).map(|local| {
                  self
                    .ast_builder
                    .import_declaration_specifier_import_default_specifier(*span, local)
                })
              }
              ImportDeclarationSpecifier::ImportNamespaceSpecifier(node) => {
                let ImportNamespaceSpecifier { span, local } = node.as_ref();
                self.transform_binding_identifier(local).map(|local| {
                  self
                    .ast_builder
                    .import_declaration_specifier_import_namespace_specifier(*span, local)
                })
              }
            };
            if let Some(specifier) = specifier {
              transformed_specifiers.push(specifier);
            }
          }
          // FIXME: side effect in module
          if transformed_specifiers.is_empty() {
            None
          } else {
            Some(self.ast_builder.module_declaration_import_declaration(
              *span,
              Some(transformed_specifiers),
              source.clone(),
              *phase,
              self.clone_node(with_clause),
              *import_kind,
            ))
          }
        } else {
          Some(self.ast_builder.module_declaration_import_declaration(
            *span,
            None,
            source.clone(),
            *phase,
            self.clone_node(with_clause),
            *import_kind,
          ))
        }
      }
      ModuleDeclaration::ExportNamedDeclaration(node) => {
        let ExportNamedDeclaration {
          span,
          declaration,
          specifiers,
          source,
          export_kind,
          with_clause,
        } = node.as_ref();
        if source.is_some() {
          // Re-exports. Nothing to do.
          return Some(ModuleDeclaration::ExportNamedDeclaration(self.clone_node(node)));
        }
        let declaration = declaration.as_ref().and_then(|d| self.transform_declaration(d));
        let mut transformed_specifiers = self.ast_builder.vec();
        for specifier in specifiers {
          if self.is_referred(AstKind2::ExportSpecifier(specifier)) {
            transformed_specifiers.push(self.clone_node(specifier));
          }
        }
        if declaration.is_none() && transformed_specifiers.is_empty() {
          return None;
        }
        Some(self.ast_builder.module_declaration_export_named_declaration(
          *span,
          declaration,
          transformed_specifiers,
          source.clone(),
          *export_kind,
          self.clone_node(with_clause),
        ))
      }
      ModuleDeclaration::ExportDefaultDeclaration(node) => {
        let ExportDefaultDeclaration { span, declaration } = node.as_ref();
        let declaration = match declaration {
          ExportDefaultDeclarationKind::FunctionDeclaration(node) => {
            ExportDefaultDeclarationKind::FunctionDeclaration(
              self.transform_function(node, true).unwrap(),
            )
          }
          ExportDefaultDeclarationKind::ClassDeclaration(node) => {
            ExportDefaultDeclarationKind::ClassDeclaration(
              self.transform_class(node, true).unwrap(),
            )
          }
          node => self.transform_expression(node.to_expression(), true).unwrap().into(),
        };
        Some(self.ast_builder.module_declaration_export_default_declaration(*span, declaration))
      }
      ModuleDeclaration::ExportAllDeclaration(node) => {
        Some(ModuleDeclaration::ExportAllDeclaration(self.clone_node(node)))
      }
      _ => unreachable!(),
    }
  }
}
