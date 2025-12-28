use oxc::ast::ast::{
  ExportDefaultDeclaration, ExportDefaultDeclarationKind, ExportNamedDeclaration,
  ImportDeclaration, ImportDeclarationSpecifier, ImportDefaultSpecifier, ImportNamespaceSpecifier,
  ImportOrExportKind, ImportSpecifier, ModuleDeclaration, ModuleExportName, Statement,
};

use crate::{
  Analyzer, ast::DeclarationKind, module::ExportedValue, transformer::Transformer,
  utils::ast::AstKind2,
};

impl<'a> Analyzer<'a> {
  pub fn declare_module_declaration(&mut self, node: &'a ModuleDeclaration<'a>) {
    let module = self.module_info_mut();
    match node {
      ModuleDeclaration::ImportDeclaration(node) => {
        if let Some(specifiers) = &node.specifiers {
          for specifier in specifiers {
            self.declare_binding_identifier(specifier.local(), None, DeclarationKind::Import);
          }
        }
      }
      ModuleDeclaration::ExportAllDeclaration(node) => {
        if let Some(exported) = &node.exported {
          // export * as exported from 'module'
          let dep = AstKind2::ExportAllDeclaration(node);
          let value =
            if let Some(module_id) = module.resolved_imports.get(&node.source.value).copied() {
              ExportedValue::Namespace(self.modules.modules[module_id].module_object, dep.into())
            } else {
              ExportedValue::Unknown(dep.into())
            };
          self.module_info_mut().named_exports.insert(exported.name(), value);
        } else {
          // export * from 'module'
          if let Some(module_id) = module.resolved_imports.get(&node.source.value) {
            module.reexport_all.insert(*module_id);
          } else {
            module.reexport_unknown = true;
          }
        }
      }
      ModuleDeclaration::ExportDefaultDeclaration(node) => {
        match &node.declaration {
          ExportDefaultDeclarationKind::FunctionDeclaration(node) => {
            let value = if node.id.is_some() {
              // Pass `exporting` as `None` because it is actually used as an expression
              self.declare_function(node, None)
            } else {
              self.exec_function(node)
            };
            self.module_info_mut().default_export = Some(Some(value));
          }
          ExportDefaultDeclarationKind::ClassDeclaration(node) => {
            if node.id.is_some() {
              // Pass `exporting` as `None` because it is actually used as an expression
              self.declare_class(node, None);
            }
          }
          _ => {}
        }
      }
      ModuleDeclaration::ExportNamedDeclaration(node) => {
        if let Some(source) = &node.source {
          // export { ... } from 'module'
          if let Some(module_id) = module.resolved_imports.get(&source.value) {
            for specifier in &node.specifiers {
              let exported = specifier.exported.name();
              let local = specifier.local.name();
              module.named_exports.insert(
                exported,
                ExportedValue::ReExport(
                  *module_id,
                  local,
                  AstKind2::ExportSpecifier(specifier).into(),
                ),
              );
            }
          } else {
            for specifier in &node.specifiers {
              let exported = specifier.exported.name();
              module.named_exports.insert(
                exported,
                ExportedValue::Unknown(AstKind2::ExportSpecifier(specifier).into()),
              );
            }
          }
        } else if let Some(declaration) = &node.declaration {
          // export let a = 1;
          self
            .declare_declaration(declaration, Some(AstKind2::ExportNamedDeclaration(node).into()));
        } else {
          // export { ... };
          for specifier in &node.specifiers {
            match &specifier.local {
              ModuleExportName::IdentifierReference(node) => {
                let dep = AstKind2::ExportSpecifier(specifier);
                let reference = self.semantic().scoping().get_reference(node.reference_id());
                if let Some(symbol) = reference.symbol_id() {
                  let scope = self.scoping.variable.top().unwrap();
                  self.module_info_mut().named_exports.insert(
                    specifier.exported.name(),
                    ExportedValue::Variable(scope, symbol, dep.into()),
                  );
                } else {
                  self.consume(dep);
                }
              }
              _ => unreachable!(),
            }
          }
        }
      }
      _ => unreachable!(),
    }
  }

  pub fn init_import_declaration(&mut self, node: &'a ImportDeclaration<'a>) {
    let name = node.source.value.as_str();
    let known = self.builtins.get_known_module(name);
    let resolved = if known.is_none() {
      self.module_info().resolved_imports.get(&node.source.value).copied()
    } else {
      None
    };

    if let Some(resolved) = resolved {
      if self.modules.modules[resolved].initializing {
        // Circular dependency
        let module = self.current_module;
        let scope = self.scoping.variable.top().unwrap();
        self.modules.modules[resolved].circular_imports.push((module, scope, node));
      }
      self.exec_module(resolved);
    }

    if let Some(specifiers) = &node.specifiers {
      for specifier in specifiers {
        let value = if let Some(known) = known {
          match specifier {
            ImportDeclarationSpecifier::ImportDefaultSpecifier(_node) => known.default,
            ImportDeclarationSpecifier::ImportNamespaceSpecifier(_node) => known.namespace,
            ImportDeclarationSpecifier::ImportSpecifier(node) => {
              let key = self.factory.unmangable_string(node.imported.name().as_str());
              known.namespace.get_property(self, self.factory.no_dep, key)
            }
          }
        } else if let Some(resolved) = resolved {
          let module = &self.modules.modules[resolved];
          match specifier {
            ImportDeclarationSpecifier::ImportDefaultSpecifier(_node) => {
              module.default_export.flatten().unwrap_or(self.factory.unknown)
            }
            ImportDeclarationSpecifier::ImportNamespaceSpecifier(_node) => module.module_object,
            ImportDeclarationSpecifier::ImportSpecifier(node) => self
              .get_export_value_by_name(resolved, node.imported.name(), &mut Default::default())
              .unwrap_or(self.factory.unknown),
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
        self.module_info_mut().default_export = Some(Some(value));
      }
      ModuleDeclaration::ExportAllDeclaration(_node) => {}
      _ => unreachable!(),
    }
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_module_declaration(
    &self,
    node: &'a ModuleDeclaration<'a>,
  ) -> Option<Statement<'a>> {
    match node {
      ModuleDeclaration::ImportDeclaration(node) => {
        let ImportDeclaration { span, specifiers, source, with_clause, import_kind, phase } =
          node.as_ref();
        if let Some(specifiers) = specifiers {
          let mut transformed_specifiers = self.ast.vec();
          for specifier in specifiers {
            let specifier = match specifier {
              ImportDeclarationSpecifier::ImportSpecifier(node) => {
                let ImportSpecifier { span, local, imported, import_kind } = node.as_ref();
                self.transform_binding_identifier(local).map(|local| {
                  self.ast.import_declaration_specifier_import_specifier(
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
                  self.ast.import_declaration_specifier_import_default_specifier(*span, local)
                })
              }
              ImportDeclarationSpecifier::ImportNamespaceSpecifier(node) => {
                let ImportNamespaceSpecifier { span, local } = node.as_ref();
                self.transform_binding_identifier(local).map(|local| {
                  self.ast.import_declaration_specifier_import_namespace_specifier(*span, local)
                })
              }
            };
            if let Some(specifier) = specifier {
              transformed_specifiers.push(specifier);
            }
          }
          Some(
            self
              .ast
              .module_declaration_import_declaration(
                *span,
                Some(transformed_specifiers),
                source.clone(),
                *phase,
                self.clone_node(with_clause),
                *import_kind,
              )
              .into(),
          )
        } else {
          Some(
            self
              .ast
              .module_declaration_import_declaration(
                *span,
                None,
                source.clone(),
                *phase,
                self.clone_node(with_clause),
                *import_kind,
              )
              .into(),
          )
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
        if let Some(declaration) = declaration {
          let need_export = self.is_deoptimized(AstKind2::ExportNamedDeclaration(node));
          let declaration = self.transform_declaration(declaration)?;
          if need_export {
            Some(
              self
                .ast
                .module_declaration_export_named_declaration(
                  *span,
                  Some(declaration),
                  self.ast.vec(),
                  source.clone(),
                  *export_kind,
                  self.clone_node(with_clause),
                )
                .into(),
            )
          } else {
            Some(declaration.into())
          }
        } else {
          let mut transformed_specifiers = self.ast.vec();
          for specifier in specifiers {
            if self.is_deoptimized(AstKind2::ExportSpecifier(specifier)) {
              transformed_specifiers.push(self.clone_node(specifier));
            }
          }
          if transformed_specifiers.is_empty() {
            source.as_ref().map(|source| {
              self
                .ast
                .module_declaration_import_declaration(
                  *span,
                  None,
                  source.clone(),
                  None,
                  self.clone_node(with_clause),
                  ImportOrExportKind::Value,
                )
                .into()
            })
          } else {
            Some(
              self
                .ast
                .module_declaration_export_named_declaration(
                  *span,
                  None,
                  transformed_specifiers,
                  source.clone(),
                  *export_kind,
                  self.clone_node(with_clause),
                )
                .into(),
            )
          }
        }
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
        Some(self.ast.module_declaration_export_default_declaration(*span, declaration).into())
      }
      ModuleDeclaration::ExportAllDeclaration(node) => {
        if node.exported.is_some() {
          let need_export = self.is_deoptimized(AstKind2::ExportAllDeclaration(node));
          need_export.then(|| ModuleDeclaration::ExportAllDeclaration(self.clone_node(node)).into())
        } else {
          Some(ModuleDeclaration::ExportAllDeclaration(self.clone_node(node)).into())
        }
      }
      _ => unreachable!(),
    }
  }
}
