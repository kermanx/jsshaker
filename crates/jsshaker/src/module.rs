use std::mem;

use line_index::LineIndex;
use oxc::{
  allocator::FromIn,
  ast::ast::{ImportDeclaration, Program, Statement},
  parser::Parser,
  semantic::{Semantic, SemanticBuilder, SymbolId},
  span::{Atom, SourceType},
};
use oxc_index::{IndexVec, define_index_type};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
  analyzer::Analyzer,
  dep::{CustomDepTrait, DepAtom},
  entity::Entity,
  scope::{VariableScopeId, variable_scope::EntityOrTDZ},
  value::module_object::ModuleObjectValue,
};

#[derive(Debug, Clone, Copy)]
pub enum ExportedValue<'a> {
  Variable(VariableScopeId, SymbolId, DepAtom),
  Function(Entity<'a>, DepAtom),
  Namespace(Entity<'a>, DepAtom),
  ReExport(ModuleId, Atom<'a>, DepAtom),
  Unknown(DepAtom),
}

impl<'a> ExportedValue<'a> {
  pub fn dep(&self) -> DepAtom {
    match self {
      ExportedValue::Variable(_, _, dep)
      | ExportedValue::Function(_, dep)
      | ExportedValue::Namespace(_, dep)
      | ExportedValue::ReExport(_, _, dep)
      | ExportedValue::Unknown(dep) => *dep,
    }
  }
}

pub struct ModuleInfo<'a> {
  pub id: ModuleId,
  pub path: Atom<'a>,
  pub line_index: LineIndex,
  pub program: &'a Program<'a>,
  pub semantic: Semantic<'a>,
  pub call_id: DepAtom,

  pub readonly_symbol_cache: FxHashMap<SymbolId, bool>,

  pub resolved_imports: FxHashMap<Atom<'a>, ModuleId>,
  pub named_exports: FxHashMap<Atom<'a>, ExportedValue<'a>>,
  pub default_export: Option<EntityOrTDZ<'a>>,
  pub reexport_all: FxHashSet<ModuleId>,
  pub reexport_unknown: bool,

  pub module_object: Entity<'a>,
  pub initializing: bool,
  pub initialized: bool,
  pub blocked_imports: Vec<(ModuleId, VariableScopeId, &'a ImportDeclaration<'a>)>,
}

define_index_type! {
  pub struct ModuleId = u32;
}

#[derive(Default)]
pub struct Modules<'a> {
  pub modules: IndexVec<ModuleId, ModuleInfo<'a>>,
  paths: FxHashMap<String, ModuleId>,
}

impl<'a> Analyzer<'a> {
  pub fn set_current_module(&mut self, module_id: ModuleId) -> ModuleId {
    mem::replace(&mut self.current_module, module_id)
  }

  pub fn module_info(&self) -> &ModuleInfo<'a> {
    &self.modules.modules[self.current_module]
  }

  pub fn module_info_mut(&mut self) -> &mut ModuleInfo<'a> {
    &mut self.modules.modules[self.current_module]
  }

  pub fn semantic<'b>(&'b self) -> &'b Semantic<'a> {
    &self.module_info().semantic
  }

  pub fn line_index(&self) -> &LineIndex {
    &self.module_info().line_index
  }

  pub fn is_readonly_symbol(&mut self, symbol_id: SymbolId) -> bool {
    let ModuleInfo { readonly_symbol_cache, semantic, .. } = self.module_info_mut();
    *readonly_symbol_cache
      .entry(symbol_id)
      .or_insert_with(|| !semantic.symbol_references(symbol_id).any(|r| r.is_write()))
  }

  pub fn resolve_and_parse_module(&mut self, specifier: &str) -> Option<ModuleId> {
    let importer = &self.module_info().path;
    let path = self.vfs.resolve_module(importer, specifier)?;
    Some(self.parse_module(path))
  }

  pub fn parse_module(&mut self, path: String) -> ModuleId {
    if let Some(module_id) = self.modules.paths.get(path.as_str()) {
      return *module_id;
    }

    let source_text = self.allocator.alloc_str(&self.vfs.read_file(path.as_str()));
    let line_index = LineIndex::new(source_text);
    let parser = Parser::new(
      self.allocator,
      source_text,
      SourceType::mjs().with_jsx(self.config.jsx.is_enabled()),
    );
    let parsed = parser.parse();
    let program = &*self.allocator.alloc(parsed.program);
    for error in parsed.errors {
      self.add_diagnostic(format!("[{}] {}", path, error));
    }
    let semantic = SemanticBuilder::new().build(program).semantic;
    let module_id: ModuleId = ModuleId::from_usize(self.modules.modules.len());
    self.modules.modules.push(ModuleInfo {
      id: module_id,
      path: Atom::from_in(path.clone(), self.allocator),
      line_index,
      program,
      semantic,
      call_id: DepAtom::from_counter(),
      readonly_symbol_cache: Default::default(),
      resolved_imports: Default::default(),
      named_exports: Default::default(),
      default_export: None,
      reexport_all: Default::default(),
      reexport_unknown: false,
      module_object: self.factory.alloc(ModuleObjectValue::new(module_id)).into(),
      initializing: false,
      initialized: false,
      blocked_imports: Default::default(),
    });
    self.modules.paths.insert(path.clone(), module_id);

    let old_module = self.set_current_module(module_id);
    for specifier in parsed.module_record.requested_modules.keys() {
      if let Some(id) = self.resolve_and_parse_module(specifier) {
        self.module_info_mut().resolved_imports.insert(*specifier, id);
      }
    }
    self.set_current_module(old_module);

    module_id
  }

  pub fn exec_module(&mut self, module_id: ModuleId) {
    let module = &mut self.modules.modules[module_id];
    if module.initialized || module.initializing {
      return;
    }
    module.initializing = true;
    let program = module.program;

    let old_module = self.set_current_module(module_id);
    for node in &program.body {
      self.declare_statement(node);
    }
    for node in &program.body {
      match node {
        Statement::ImportDeclaration(node) => {
          self.init_import_declaration(node);
        }
        Statement::ExportAllDeclaration(node) => {
          if let Some(resolved) = self.module_info().resolved_imports.get(&node.source.value) {
            self.exec_module(*resolved);
          }
        }
        Statement::ExportNamedDeclaration(node) => {
          if let Some(source) = &node.source
            && let Some(resolved) = self.module_info().resolved_imports.get(&source.value)
          {
            self.exec_module(*resolved);
          }
        }
        _ => {}
      }
    }
    for node in &program.body {
      self.init_statement(node);
    }

    let module = self.module_info_mut();
    module.initializing = false;
    module.initialized = true;

    for (module, scope, node) in mem::take(&mut module.blocked_imports) {
      self.set_current_module(module);
      let old_scope = self.replace_variable_scope(Some(scope));
      self.init_import_declaration(node);
      self.replace_variable_scope(old_scope);
    }

    self.set_current_module(old_module);
  }

  pub fn consume_exports(&mut self, module_id: ModuleId) {
    let module = &self.modules.modules[module_id];
    let call_id = module.call_id;
    let mut values = vec![];
    if let Some(Some(default_export)) = module.default_export {
      values.push(default_export);
    }
    let named_exports = module.named_exports.values().copied().collect::<Vec<_>>();
    for named_export in named_exports {
      values.push(self.get_named_export_value(module_id, named_export));
    }
    self.consume((call_id, values));
  }

  pub fn get_named_export_value(
    &mut self,
    module_id: ModuleId,
    named_export: ExportedValue<'a>,
  ) -> Entity<'a> {
    match named_export {
      ExportedValue::Variable(scope, symbol, dep) => {
        let old_module = self.set_current_module(module_id);
        let value = self.read_on_scope(scope, symbol).unwrap().unwrap_or(self.factory.unknown);
        self.set_current_module(old_module);
        self.factory.computed(value, dep)
      }
      ExportedValue::Function(entity, dep) => self.factory.computed(entity, dep),
      ExportedValue::Namespace(entity, dep) => self.factory.computed(entity, dep),
      ExportedValue::ReExport(module, name, dep) => {
        let value = self
          .get_export_value_by_name(module, name, &mut FxHashSet::default())
          .unwrap_or(self.factory.unknown);
        self.factory.computed(value, dep)
      }
      ExportedValue::Unknown(dep) => self.factory.computed_unknown(dep),
    }
  }

  pub fn get_export_value_by_name(
    &mut self,
    module_id: ModuleId,
    name: Atom<'a>,
    searched: &mut FxHashSet<ModuleId>,
  ) -> Option<Entity<'a>> {
    if !searched.insert(module_id) {
      return None;
    }
    let module = &self.modules.modules[module_id];
    if name == "default" {
      module.default_export.map(|e| e.unwrap_or(self.factory.unknown))
    } else if let Some(exported_value) = module.named_exports.get(&name) {
      Some(self.get_named_export_value(module_id, *exported_value))
    } else {
      for reexport_module_id in module.reexport_all.clone() {
        if let Some(entity) = self.get_export_value_by_name(reexport_module_id, name, searched) {
          return Some(entity);
        }
      }
      None
    }
  }

  pub fn does_module_reexport_unknown(
    &self,
    module_id: ModuleId,
    searched: &mut FxHashSet<ModuleId>,
  ) -> bool {
    if !searched.insert(module_id) {
      return false;
    }
    let module = &self.modules.modules[module_id];
    if module.reexport_unknown {
      return true;
    }
    for reexport_module_id in module.reexport_all.iter() {
      if self.does_module_reexport_unknown(*reexport_module_id, searched) {
        return true;
      }
    }
    false
  }

  pub fn get_exported_keys(
    &self,
    module_id: ModuleId,
    searched: &mut FxHashSet<ModuleId>,
  ) -> Vec<Entity<'a>> {
    if !searched.insert(module_id) {
      return vec![];
    }
    let module = &self.modules.modules[module_id];
    let mut keys = vec![];
    for reexport_module_id in module.reexport_all.iter() {
      let reexported_keys = self.get_exported_keys(*reexport_module_id, searched);
      for key in reexported_keys {
        keys.push(key);
      }
    }
    for (name, value) in &module.named_exports {
      keys.push(self.factory.computed(self.factory.unmangable_string(name.as_str()), value.dep()));
    }
    if searched.is_empty()
      && let Some(default_export) = &module.default_export
      && default_export.is_some()
    {
      keys.push(self.factory.computed(self.factory.builtin_string("default"), *default_export));
    }

    keys
  }
}

impl CustomDepTrait<'_> for ModuleId {
  fn consume(&self, analyzer: &mut Analyzer) {
    analyzer.consume_exports(*self);
  }
}
