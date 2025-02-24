extern crate console_error_panic_hook;

use oxc::{
  codegen::CodegenOptions,
  minifier::{MangleOptions, MinifierOptions},
};
use tree_shaker::vfs::SingleFileFs;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(getter_with_clone)]
pub struct Result {
  pub output: String,
  pub diagnostics: Vec<String>,
}

#[wasm_bindgen]
pub fn tree_shake(
  source_text: String,
  preset: String,
  do_minify: bool,
  always_inline_literal: bool,
) -> Result {
  console_error_panic_hook::set_once();

  let result = tree_shaker::tree_shake(
    tree_shaker::TreeShakeOptions {
      vfs: SingleFileFs(source_text),
      config: match preset.as_str() {
        "recommended" => tree_shaker::TreeShakeConfig::recommended(),
        "smallest" => tree_shaker::TreeShakeConfig::smallest(),
        "safest" => tree_shaker::TreeShakeConfig::safest(),
        "disabled" => tree_shaker::TreeShakeConfig::disabled(),
        _ => unreachable!("Invalid preset {}", preset),
      }
      .with_react_jsx(true)
      .with_always_inline_literal(always_inline_literal),
      minify_options: do_minify.then_some({
        MinifierOptions {
          mangle: Some(MangleOptions { top_level: true, ..Default::default() }),
          ..Default::default()
        }
      }),
      codegen_options: CodegenOptions {
        minify: do_minify,
        comments: !do_minify,
        ..Default::default()
      },
    },
    SingleFileFs::ENTRY_PATH.to_string(),
  );
  Result {
    output: result.codegen_return[SingleFileFs::ENTRY_PATH].code.clone(),
    diagnostics: result.diagnostics.into_iter().collect(),
  }
}
