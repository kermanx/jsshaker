extern crate console_error_panic_hook;

use jsshaker::vfs::SingleFileFs;
use oxc::{
  codegen::{CodegenOptions, CommentOptions},
  minifier::{MangleOptions, MinifierOptions},
};
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

  let result = jsshaker::tree_shake(
    jsshaker::JsShakerOptions {
      vfs: SingleFileFs(source_text),
      config: match preset.as_str() {
        "recommended" => jsshaker::TreeShakeConfig::recommended(),
        "smallest" => jsshaker::TreeShakeConfig::smallest(),
        "safest" => jsshaker::TreeShakeConfig::safest(),
        "disabled" => jsshaker::TreeShakeConfig::disabled(),
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
        comments: if do_minify { CommentOptions::disabled() } else { CommentOptions::default() },
        ..Default::default()
      },
      source_map: false,
    },
    SingleFileFs::ENTRY_PATH.to_string(),
  );
  Result {
    output: result.codegen_return[SingleFileFs::ENTRY_PATH].code.clone(),
    diagnostics: result.diagnostics.into_iter().collect(),
  }
}
