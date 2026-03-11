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
  #[napi(ts_type = "'react'")]
  pub jsx: Option<String>,
  pub source_map: Option<bool>,

  #[napi(ts_type = "'enabled' | 'disabled' | 'all'")]
  pub constant_folding: Option<String>,
  #[napi(ts_type = "'enabled' | 'disabled' | 'only'")]
  pub property_mangling: Option<String>,
  pub branch_folding: Option<bool>,

  pub max_recursion_depth: Option<u32>,
  pub remember_exhausted_variables: Option<bool>,
  pub eager_exhaustive_callbacks: Option<bool>,
  pub enable_fn_cache: Option<bool>,
  pub enable_fn_stats: Option<bool>,
  pub enable_mangling_stats: Option<bool>,
}

#[napi(object)]
pub struct Chunk {
  pub code: String,
  pub source_map_json: Option<String>,
}

#[napi(object)]
pub struct Stat {
  pub fn_cache: Option<FnCacheStat>,
  pub mangling: Option<ManglingStat>,
}

#[napi(object)]
pub struct FnCacheStat {
  pub total_calls: u32,
  pub cache_attempts: u32,
  pub cache_hits: u32,
  pub cache_updates: u32,

  // Function counts
  pub function_declarations: u32,
  pub function_instances: u32,

  // Miss reason breakdown
  pub miss_config_disabled: u32,
  pub miss_non_copyable_this: u32,
  pub miss_non_copyable_args: u32,
  pub miss_rest_params: u32,
  pub miss_non_copyable_return: u32,
  pub miss_state_untrackable: u32,
  pub miss_read_dep_incompatible: u32,
  pub miss_cache_empty: u32,
}

#[napi(object)]
pub struct ManglingStat {
  pub dynamic: u32,
  pub static_all: u32,
  pub static_mangled: u32,
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
  if options.jsx.as_deref() == Some("react") {
    config.jsx = TreeShakeJsxPreset::React;
  }

  if let Some(constant_folding) = options.constant_folding.as_deref() {
    match constant_folding {
      "enabled" => {}
      "disabled" => config.folding = false,
      "all" => config.max_folding_string_length = usize::MAX,
      _ => panic!("Invalid constant_folding option {:?}", constant_folding),
    };
  }
  if let Some(property_mangling) = options.property_mangling.as_deref() {
    config.mangling = match property_mangling {
      "enabled" => Some(false),
      "disabled" => None,
      "only" => Some(true),
      _ => panic!("Invalid property_mangling option {:?}", property_mangling),
    };
  }
  if let Some(branch_folding) = options.branch_folding {
    config.branch_folding = branch_folding;
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
  if let Some(enable) = options.enable_mangling_stats {
    config.enable_mangling_stats = enable;
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

fn convert_stats(
  fn_stats: Option<jsshaker::FnStats>,
  mangling_stats: Option<jsshaker::ManglingStats>,
) -> Stat {
  Stat {
    fn_cache: fn_stats.map(|stats| {
      let overall = &stats.overall;
      FnCacheStat {
        total_calls: overall.total_calls as u32,
        cache_attempts: overall.cache_attempts as u32,
        cache_hits: overall.cache_hits as u32,
        cache_updates: overall.cache_updates as u32,
        function_declarations: overall.function_declarations.len() as u32,
        function_instances: overall.function_instances as u32,
        miss_config_disabled: overall.miss_config_disabled as u32,
        miss_non_copyable_this: overall.miss_non_copyable_this as u32,
        miss_non_copyable_args: overall.miss_non_copyable_args as u32,
        miss_rest_params: overall.miss_rest_params as u32,
        miss_non_copyable_return: overall.miss_non_copyable_return as u32,
        miss_state_untrackable: overall.miss_state_untrackable as u32,
        miss_read_dep_incompatible: overall.miss_read_dep_incompatible as u32,
        miss_cache_empty: overall.miss_cache_empty as u32,
      }
    }),
    mangling: mangling_stats.map(|stats| ManglingStat {
      dynamic: stats.dynamic as u32,
      static_all: stats.static_all as u32,
      static_mangled: stats.static_mangled as u32,
    }),
  }
}

#[napi(object)]
pub struct SingleModuleResult {
  pub output: Chunk,
  pub diagnostics: Vec<String>,
  pub stat: Stat,
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
    stat: convert_stats(result.fn_stats, result.mangling_stats),
  }
}

#[napi(object)]
pub struct MultiModuleResult {
  pub output: HashMap<String, Chunk>,
  pub diagnostics: Vec<String>,
  pub stat: Stat,
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
  MultiModuleResult {
    output,
    diagnostics: result.diagnostics.into_iter().collect(),
    stat: convert_stats(result.fn_stats, result.mangling_stats),
  }
}

#[napi]
pub fn shake_fs_module(entry_path: String, options: Options) -> MultiModuleResult {
  let result = jsshaker::tree_shake(resolve_options(StdFs, options), entry_path.clone());
  let mut output = HashMap::default();
  for (entry, codegen_result) in result.codegen_return {
    output.insert(entry, codegen_result.into());
  }
  MultiModuleResult {
    output,
    diagnostics: result.diagnostics.into_iter().collect(),
    stat: convert_stats(result.fn_stats, result.mangling_stats),
  }
}
