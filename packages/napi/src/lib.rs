#![deny(clippy::all)]

use std::collections::HashMap;

use jsshaker::{
  JsShakerOptions, TreeShakeConfig, TreeShakeJsxPreset,
  vfs::{MultiModuleFs, SingleFileFs, StdFs, Vfs},
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
  pub source_map: Option<bool>,

  pub max_recursion_depth: Option<u32>,
  pub remember_exhausted_variables: Option<bool>,
  pub eager_exhaustive_callbacks: Option<bool>,
  pub enable_fn_cache: Option<bool>,
  pub enable_fn_stats: Option<bool>,
}

#[napi(object)]
pub struct Chunk {
  pub code: String,
  pub source_map_json: Option<String>,
}

impl From<oxc::codegen::CodegenReturn> for Chunk {
  fn from(value: oxc::codegen::CodegenReturn) -> Self {
    Chunk { code: value.code, source_map_json: value.map.map(|m| m.to_json_string()) }
  }
}

fn resolve_options<F: Vfs>(vfs: F, options: Options) -> JsShakerOptions<F> {
  let preset = options.preset.as_deref().unwrap_or("recommended");

  let mut config = match preset {
    "safest" => TreeShakeConfig::safest(),
    "recommended" => TreeShakeConfig::recommended(),
    "smallest" => TreeShakeConfig::smallest(),
    "disabled" => TreeShakeConfig::disabled(),
    _ => panic!("Invalid tree shake option {:?}", preset),
  };
  if options.always_inline_literal == Some(true) {
    config.min_simple_number_value = i64::MIN;
    config.max_simple_number_value = i64::MAX;
    config.max_simple_string_length = usize::MAX;
  }
  if options.jsx.as_deref() == Some("react") {
    config.jsx = TreeShakeJsxPreset::React;
  }
  if let Some(depth) = options.max_recursion_depth {
    config.max_recursion_depth = depth as usize;
  }
  if let Some(remember) = options.remember_exhausted_variables {
    config.remember_exhausted_variables = remember;
  }
  if let Some(eager) = options.eager_exhaustive_callbacks {
    config.eager_exhaustive_callbacks = eager;
  }
  if let Some(enable) = options.enable_fn_cache {
    config.enable_fn_cache = enable;
  }
  if let Some(enable) = options.enable_fn_stats {
    config.enable_fn_stats = enable;
  }

  let minify = options.minify.unwrap_or(false);
  let minify_options =
    if minify { Some(MinifierOptions { mangle: None, ..Default::default() }) } else { None };

  JsShakerOptions {
    vfs,
    config,
    minify_options,
    codegen_options: CodegenOptions { minify, ..Default::default() },
    source_map: options.source_map.unwrap_or(false),
  }
}

#[napi(object)]
pub struct SingleModuleResult {
  pub output: Chunk,
  pub diagnostics: Vec<String>,
}

#[napi]
pub fn shake_single_module(source_text: String, options: Options) -> SingleModuleResult {
  let mut result = jsshaker::tree_shake(
    resolve_options(SingleFileFs(source_text), options),
    SingleFileFs::ENTRY_PATH.to_string(),
  );
  SingleModuleResult {
    output: result.codegen_return.remove(SingleFileFs::ENTRY_PATH).unwrap().into(),
    diagnostics: result.diagnostics.into_iter().collect(),
  }
}

#[napi(object)]
pub struct MultiModuleResult {
  pub output: HashMap<String, Chunk>,
  pub diagnostics: Vec<String>,
}

#[napi]
pub fn shake_multi_module(
  sources: HashMap<String, String>,
  entry: String,
  options: Options,
) -> MultiModuleResult {
  let result = jsshaker::tree_shake(resolve_options(MultiModuleFs(sources), options), entry);
  let mut output = HashMap::default();
  for (entry, codegen_result) in result.codegen_return {
    output.insert(entry, codegen_result.into());
  }
  MultiModuleResult { output, diagnostics: result.diagnostics.into_iter().collect() }
}

#[napi]
pub fn shake_fs_module(entry_path: String, options: Options) -> MultiModuleResult {
  let result = jsshaker::tree_shake(resolve_options(StdFs, options), entry_path.clone());
  let mut output = HashMap::default();
  for (entry, codegen_result) in result.codegen_return {
    output.insert(entry, codegen_result.into());
  }
  MultiModuleResult { output, diagnostics: result.diagnostics.into_iter().collect() }
}
