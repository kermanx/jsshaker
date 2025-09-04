#![deny(clippy::all)]

use std::collections::HashMap;

use jsshaker::{
  JsShakerOptions,
  vfs::{SingleFileFs, StdFs, Vfs},
};
use napi_derive::napi;
use oxc::{codegen::CodegenOptions, minifier::MinifierOptions};

#[napi(object)]
pub struct Options {
  #[napi(ts_type = "'safest' | 'recommended' | 'smallest' | 'disabled'")]
  pub preset: Option<String>,
  pub minify: Option<bool>,
  pub always_inline_literal: Option<bool>,
  #[napi(ts_type = "'react'")]
  pub jsx: Option<String>,
}

fn resolve_options<F: Vfs>(vfs: F, options: Options) -> JsShakerOptions<F> {
  let preset = options.preset.as_deref().unwrap_or("recommended");
  let minify = options.minify.unwrap_or(false);
  let always_inline_literal = options.always_inline_literal.unwrap_or(false);
  JsShakerOptions {
    vfs,
    config: match preset {
      "safest" => jsshaker::TreeShakeConfig::safest(),
      "recommended" => jsshaker::TreeShakeConfig::recommended(),
      "smallest" => jsshaker::TreeShakeConfig::smallest(),
      "disabled" => jsshaker::TreeShakeConfig::disabled(),
      _ => panic!("Invalid tree shake option {:?}", preset),
    }
    .with_always_inline_literal(always_inline_literal)
    .with_react_jsx(options.jsx.as_deref() == Some("react")),
    minify_options: minify.then(|| MinifierOptions { mangle: None, ..Default::default() }),
    codegen_options: CodegenOptions { minify, ..Default::default() },
  }
}

#[napi(object)]
pub struct SingleModuleResult {
  pub output: String,
  pub diagnostics: Vec<String>,
}

#[napi]
pub fn shake_single_module(source_text: String, options: Options) -> SingleModuleResult {
  let result = jsshaker::tree_shake(
    resolve_options(SingleFileFs(source_text), options),
    SingleFileFs::ENTRY_PATH.to_string(),
  );
  SingleModuleResult {
    output: result.codegen_return[SingleFileFs::ENTRY_PATH].code.clone(),
    diagnostics: result.diagnostics.into_iter().collect(),
  }
}

#[napi(object)]
pub struct MultiModuleResult {
  pub output: HashMap<String, String>,
  pub diagnostics: Vec<String>,
}

#[napi]
pub fn shake_multi_module(entry_path: String, options: Options) -> MultiModuleResult {
  let result = jsshaker::tree_shake(resolve_options(StdFs, options), entry_path.clone());
  let mut output = HashMap::default();
  for (entry, codegen_result) in result.codegen_return {
    output.insert(entry, codegen_result.code);
  }
  MultiModuleResult { output, diagnostics: result.diagnostics.into_iter().collect() }
}
