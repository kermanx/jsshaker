use std::mem;

use line_index::LineIndex;
use oxc::{
  allocator::FromIn,
  ast::ast::{Program, PropertyKind},
  parser::Parser,
  semantic::{Semantic, SemanticBuilder, SymbolId},
  span::{Atom, SourceType},
};
use oxc_index::{define_index_type, IndexVec};
use rustc_hash::FxHashMap;

use crate::{
  analyzer::Analyzer,
  entity::{Entity, ObjectEntity},
  scope::{call_scope::CallScope, cf_scope::CfScope, CfScopeId, CfScopeKind},
  utils::{dep_id::DepId, CalleeInfo, CalleeNode},
};

pub struct ModuleInfo<'a> {
  pub path: Atom<'a>,
  pub line_index: LineIndex,
  pub program: &'a Program<'a>,
  pub semantic: &'a Semantic<'a>,
  pub call_id: DepId,
  pub export_object: Option<&'a ObjectEntity<'a>>,

  pub pending_named_exports: FxHashMap<Atom<'a>, SymbolId>,
  pub pending_default_export: Option<Entity<'a>>,
  pub pending_reexports: Vec<Entity<'a>>,
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
  pub fn module_info(&self) -> &ModuleInfo<'a> {
    &self.modules.modules[self.current_module()]
  }

  pub fn module_info_mut(&mut self) -> &mut ModuleInfo<'a> {
    let module_id = self.current_module();
    &mut self.modules.modules[module_id]
  }

  pub fn semantic(&self) -> &'a Semantic<'a> {
    self.module_info().semantic
  }

  pub fn line_index(&self) -> &LineIndex {
    &self.module_info().line_index
  }

  pub fn resolve_and_import_module(&mut self, specifier: &str) -> Option<ModuleId> {
    let importer = &self.module_info().path;
    let path = self.vfs.resolve_module(importer, specifier)?;
    Some(self.import_module(path))
  }

  pub fn import_module(&mut self, path: String) -> ModuleId {
    let path = self.vfs.normalize_path(path);

    if let Some(module_id) = self.modules.paths.get(path.as_str()) {
      return *module_id;
    }

    let source_text = self.allocator.alloc(self.vfs.read_file(path.as_str()));
    let line_index = LineIndex::new(source_text);
    let parser = Parser::new(
      self.allocator,
      source_text,
      SourceType::mjs().with_jsx(self.config.jsx.is_enabled()),
    );
    let parsed = parser.parse();
    let program = self.allocator.alloc(parsed.program);
    for error in parsed.errors {
      self.add_diagnostic(format!("[{}] {}", path, error));
    }
    let semantic = SemanticBuilder::new().build(program).semantic;
    let semantic = self.allocator.alloc(semantic);
    let call_id = DepId::from_counter();
    let module_id = self.modules.modules.push(ModuleInfo {
      path: Atom::from_in(path.clone(), self.allocator),
      line_index,
      program,
      semantic,
      call_id,
      export_object: None,

      pending_named_exports: Default::default(),
      pending_default_export: Default::default(),
      pending_reexports: Default::default(),
    });
    self.modules.paths.insert(path.clone(), module_id);

    self.module_stack.push(module_id);
    let old_variable_scope_stack = self.replace_variable_scope_stack(vec![]);
    let root_variable_scope = self.push_variable_scope();
    self.scope_context.call.push(CallScope::new(
      call_id,
      CalleeInfo {
        module_id,
        node: CalleeNode::Module,
        instance_id: self.factory.alloc_instance_id(),
        #[cfg(feature = "flame")]
        debug_name: "<Module>",
      },
      vec![],
      0,
      root_variable_scope,
      true,
      false,
    ));
    let old_cf_scope_stack = self.scope_context.cf.replace_stack(vec![CfScopeId::from(0)]);
    self.scope_context.cf.push(CfScope::new(CfScopeKind::Module, vec![], Some(false)));

    self.init_exports();
    self.exec_program(program);
    self.finalize_exports();

    self.scope_context.cf.replace_stack(old_cf_scope_stack);
    self.scope_context.call.pop();
    self.replace_variable_scope_stack(old_variable_scope_stack);
    self.module_stack.pop();

    module_id
  }

  fn init_exports(&mut self) {
    let export_object = self.new_empty_object(&self.builtins.prototypes.null, None);
    self.module_info_mut().export_object = Some(export_object);
  }

  fn finalize_exports(&mut self) {
    let export_object = self.module_info_mut().export_object.unwrap();
    for (name, symbol) in mem::take(&mut self.module_info_mut().pending_named_exports) {
      let value = self.read_symbol(symbol).unwrap();
      export_object.init_property(
        self,
        PropertyKind::Init,
        self.factory.string(name.as_str()),
        value,
        true,
      );
    }
    if let Some(default_export) = mem::take(&mut self.module_info_mut().pending_default_export) {
      export_object.init_property(
        self,
        PropertyKind::Init,
        self.factory.string("default"),
        default_export,
        true,
      );
    }
    for reexport in mem::take(&mut self.module_info_mut().pending_reexports) {
      export_object.init_spread(self, self.factory.empty_consumable, reexport);
    }
  }

  pub fn consume_exports(&mut self, module_id: ModuleId) {
    let ModuleInfo { call_id, export_object, .. } = &self.modules.modules[module_id];
    let call_id = *call_id;
    self.consume(export_object.unwrap());
    self.refer_dep(call_id);
  }
}
