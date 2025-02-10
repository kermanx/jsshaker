use crate::{
  analyzer::Analyzer, entity::EntityFactory, transformer::Transformer, utils::Diagnostics,
  TreeShakeConfig,
};
use oxc::{
  allocator::Allocator,
  ast::ast::Program,
  codegen::{CodeGenerator, CodegenOptions, CodegenReturn},
  minifier::{Minifier, MinifierOptions},
  semantic::SemanticBuilder,
};
use std::{cell::RefCell, collections::BTreeSet, rc::Rc};

pub struct TreeShakeOptions {
  pub config: TreeShakeConfig,
  pub minify_options: Option<MinifierOptions>,
  pub codegen_options: CodegenOptions,
}

pub struct TreeShakeReturn {
  pub codegen_return: CodegenReturn,
  pub diagnostics: Diagnostics,
}

pub struct TreeShakerInner<'a> {
  pub allocator: &'a Allocator,
  pub config: &'a TreeShakeConfig,
  pub factory: &'a EntityFactory<'a>,
  pub minify_options: Option<MinifierOptions>,
  pub codegen_options: CodegenOptions,
  pub diagnostics: RefCell<BTreeSet<String>>,
}

#[derive(Clone)]
pub struct TreeShaker<'a>(pub Rc<TreeShakerInner<'a>>);

impl<'a> TreeShaker<'a> {
  pub fn new(allocator: &'a Allocator, options: TreeShakeOptions) -> Self {
    let TreeShakeOptions { config, minify_options, codegen_options } = options;

    let config = allocator.alloc(config);
    let factory = allocator.alloc(EntityFactory::new(allocator, config));

    Self(Rc::new(TreeShakerInner {
      allocator,
      config,
      minify_options,
      codegen_options,
      factory,
      diagnostics: Default::default(),
    }))
  }

  pub fn tree_shake(self, ast: &'a mut Program<'a>) -> TreeShakeReturn {
    let TreeShakerInner { allocator, config, minify_options, codegen_options, .. } = &*self.0;

    let ast = if config.enabled {
      let semantic_builder = SemanticBuilder::new();
      let semantic = semantic_builder.build(ast).semantic;

      // Step 1: Analyze the program
      let mut analyzer = Analyzer::new(self.clone(), semantic);
      analyzer.exec_program(ast);

      // Step 2: Remove dead code (transform)
      let transformer = Transformer::new(analyzer);
      allocator.alloc(transformer.transform_program(ast))
    } else {
      ast
    };

    // Step 3: Minify
    let minifier_return = minify_options.map(|options| {
      let minifier = Minifier::new(options);
      minifier.build(allocator, ast)
    });

    // Step 4: Generate output
    let codegen = CodeGenerator::new()
      .with_options(codegen_options.clone())
      .with_symbol_table(minifier_return.and_then(|r| r.symbol_table));
    let codegen_return = codegen.build(ast);

    TreeShakeReturn { codegen_return, diagnostics: self.0.diagnostics.take() }
  }
}
