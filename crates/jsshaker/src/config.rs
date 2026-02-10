use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeShakeJsxPreset {
  None,
  React,
}

impl TreeShakeJsxPreset {
  pub fn is_enabled(&self) -> bool {
    *self != Self::None
  }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TreeShakeConfig {
  pub enabled: bool,
  pub jsx: TreeShakeJsxPreset,

  pub max_recursion_depth: usize,
  pub remember_exhausted_variables: bool,
  pub eager_exhaustive_callbacks: bool,
  pub enable_fn_cache: bool,
  pub enable_fn_stats: bool,

  pub folding: bool,
  pub max_folding_string_length: usize,
  pub mangling: Option<bool>,

  pub unknown_global_side_effects: bool,
  pub preserve_function_name: bool,
  pub preserve_function_length: bool,
  pub iterate_side_effects: bool,
  pub unknown_property_read_side_effects: bool,
  pub unmatched_prototype_property_as_undefined: bool,
  pub preserve_exceptions: bool,
  pub preserve_property_attributes: bool, // enumerable, configurable, writable
  pub impure_json_stringify: bool,
  pub precise_dynamic_prototype: bool,
}

impl Default for TreeShakeConfig {
  fn default() -> Self {
    Self::safest()
  }
}

impl TreeShakeConfig {
  pub fn safest() -> Self {
    Self {
      enabled: true,
      jsx: TreeShakeJsxPreset::None,

      max_recursion_depth: 2,
      remember_exhausted_variables: true,
      eager_exhaustive_callbacks: false,
      enable_fn_cache: true,
      enable_fn_stats: false,

      folding: true,
      max_folding_string_length: 12,
      mangling: Some(false),

      unknown_global_side_effects: true,
      preserve_function_name: true,
      preserve_function_length: true,
      iterate_side_effects: true,
      unknown_property_read_side_effects: true,
      unmatched_prototype_property_as_undefined: false,
      preserve_exceptions: true,
      preserve_property_attributes: true,
      impure_json_stringify: true,
      precise_dynamic_prototype: true,
    }
  }

  pub fn recommended() -> Self {
    Self {
      unknown_global_side_effects: false,
      preserve_function_name: false,
      preserve_function_length: false,
      preserve_exceptions: false,
      preserve_property_attributes: false,
      impure_json_stringify: false,
      precise_dynamic_prototype: false,

      ..Self::default()
    }
  }

  pub fn smallest() -> Self {
    Self {
      iterate_side_effects: false,
      unknown_property_read_side_effects: false,
      unmatched_prototype_property_as_undefined: true,

      ..Self::recommended()
    }
  }

  pub fn disabled() -> Self {
    Self { enabled: false, ..Self::default() }
  }
}
