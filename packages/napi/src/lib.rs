#![deny(clippy::all)]

use std::collections::HashMap;

use napi_derive::napi;
use oxc::{codegen::CodegenOptions, minifier::MinifierOptions};
use tree_shaker::{
  TreeShakeOptions,
  vfs::{SingleFileFs, StdFs, Vfs},
};

#[napi]
pub struct TreeShakeResultBinding {
  pub output: String,
  pub diagnostics: Vec<String>,
}

#[napi(
  ts_args_type = "input: string, preset: 'safest' | 'recommended' | 'smallest' | 'disabled', minify: boolean"
)]
pub fn tree_shake(source_text: String, preset: String, minify: bool) -> TreeShakeResultBinding {
  let result = tree_shaker::tree_shake(
    get_options(SingleFileFs(source_text), preset.as_str(), minify),
    SingleFileFs::ENTRY_PATH.to_string(),
  );
  TreeShakeResultBinding {
    output: result.codegen_return[SingleFileFs::ENTRY_PATH].code.clone(),
    diagnostics: result.diagnostics.into_iter().collect(),
  }
}

#[napi(object)]
pub struct TreeShakeEntryResultBinding {
  pub output: HashMap<String, String>,
  pub diagnostics: Vec<String>,
}

#[napi(
  ts_args_type = "entryPath: string, preset: 'safest' | 'recommended' | 'smallest' | 'disabled', minify: boolean"
)]
pub fn tree_shake_entry(
  entry_path: String,
  preset: String,
  minify: bool,
) -> TreeShakeEntryResultBinding {
  let result =
    tree_shaker::tree_shake(get_options(StdFs, preset.as_str(), minify), entry_path.clone());
  let mut output = HashMap::default();
  for (entry, codegen_result) in result.codegen_return {
    output.insert(entry, codegen_result.code);
  }
  TreeShakeEntryResultBinding { output, diagnostics: result.diagnostics.into_iter().collect() }
}

fn get_options<F: Vfs>(vfs: F, preset: &str, minify: bool) -> TreeShakeOptions<F> {
  TreeShakeOptions {
    vfs,
    config: match preset {
      "safest" => tree_shaker::TreeShakeConfig::safest(),
      "recommended" => tree_shaker::TreeShakeConfig::recommended(),
      "smallest" => tree_shaker::TreeShakeConfig::smallest(),
      "disabled" => tree_shaker::TreeShakeConfig::disabled(),
      _ => panic!("Invalid tree shake option {}", preset),
    },
    minify_options: minify.then(|| MinifierOptions { mangle: None, ..Default::default() }),
    codegen_options: CodegenOptions { minify, ..Default::default() },
  }
}
