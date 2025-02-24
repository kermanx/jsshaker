mod analyzer;
mod builtins;
mod config;
mod consumable;
mod entity;
mod mangling;
mod module;
mod nodes;
mod scope;
mod transformer;
mod utils;
pub mod vfs;

use std::{cell::RefCell, mem, rc::Rc};

use analyzer::Analyzer;
use module::ModuleInfo;
use oxc::{
  allocator::Allocator,
  codegen::{CodeGenerator, CodegenOptions, CodegenReturn},
  minifier::{Minifier, MinifierOptions},
};
use rustc_hash::FxHashMap;
use transformer::Transformer;
use utils::{
  ast,
  dep_id::{self as dep},
  Diagnostics,
};

pub use config::{TreeShakeConfig, TreeShakeJsxPreset};
use vfs::Vfs;

pub struct TreeShakeOptions<F: Vfs> {
  pub vfs: F,
  pub config: TreeShakeConfig,
  pub minify_options: Option<MinifierOptions>,
  pub codegen_options: CodegenOptions,
}

pub struct TreeShakeReturn {
  pub codegen_return: FxHashMap<String, CodegenReturn>,
  pub diagnostics: Diagnostics,
}

pub fn tree_shake<F: Vfs>(options: TreeShakeOptions<F>, entry: String) -> TreeShakeReturn {
  let TreeShakeOptions { vfs, config, minify_options, codegen_options } = options;
  let allocator = Allocator::default();
  let config = allocator.alloc(config);

  // Step 1: Analyze
  let analyzer = Analyzer::new_in(allocator.alloc(vfs), config, &allocator);
  analyzer.import_module(entry);
  analyzer.finalize();
  let Analyzer { modules, diagnostics, mangler, data, referred_deps, conditional_data, .. } =
    analyzer;
  let mangler = Rc::new(RefCell::new(mangler));
  let mut codegen_return = FxHashMap::default();
  for module_info in &modules.modules {
    let ModuleInfo { path, program, semantic, .. } = module_info;

    // Setp 2: Transform
    let transformer = Transformer::new(
      config,
      &allocator,
      data,
      referred_deps,
      conditional_data,
      mangler.clone(),
      semantic,
    );
    let program = allocator.alloc(transformer.transform_program(program));

    // Step 3: Minify
    let minifier_return = minify_options.map(|options| {
      let minifier = Minifier::new(options);
      minifier.build(&allocator, program)
    });

    // Step 4: Generate output
    let codegen = CodeGenerator::new()
      .with_options(codegen_options.clone())
      .with_mangler(minifier_return.and_then(|r| r.mangler));
    codegen_return.insert(path.to_string(), codegen.build(program));
  }

  TreeShakeReturn { codegen_return, diagnostics: mem::take(diagnostics) }
}
