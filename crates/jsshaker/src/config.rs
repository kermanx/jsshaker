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

  pub mangling: Option<bool>,
  pub unknown_global_side_effects: bool,
  pub preserve_function_name: bool,
  pub preserve_function_length: bool,
  pub iterate_side_effects: bool,
  pub unknown_property_read_side_effects: bool,
  pub unmatched_prototype_property_as_undefined: bool,
  pub preserve_writablity: bool,
  pub preserve_exceptions: bool,
  pub impure_json_stringify: bool,

  pub min_simple_number_value: i64,
  pub max_simple_number_value: i64,
  pub max_simple_string_length: usize,
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

      mangling: Some(false),
      unknown_global_side_effects: true,
      preserve_function_name: true,
      preserve_function_length: true,
      iterate_side_effects: true,
      unknown_property_read_side_effects: true,
      unmatched_prototype_property_as_undefined: false,
      preserve_writablity: true,
      preserve_exceptions: true,
      impure_json_stringify: true,

      min_simple_number_value: -1_000_000,
      max_simple_number_value: 1_000_000,
      max_simple_string_length: 12,
    }
  }

  pub fn recommended() -> Self {
    Self {
      preserve_function_name: false,
      preserve_function_length: false,
      preserve_writablity: false,
      preserve_exceptions: false,
      impure_json_stringify: false,

      ..Default::default()
    }
  }

  pub fn smallest() -> Self {
    Self {
      unknown_global_side_effects: false,
      preserve_function_name: false,
      preserve_function_length: false,
      iterate_side_effects: false,
      unknown_property_read_side_effects: false,
      unmatched_prototype_property_as_undefined: true,
      preserve_writablity: false,
      preserve_exceptions: false,

      ..Default::default()
    }
  }

  pub fn disabled() -> Self {
    Self { enabled: false, ..Default::default() }
  }

  pub fn with_always_inline_literal(mut self, yes: bool) -> Self {
    if yes {
      self.min_simple_number_value = i64::MIN;
      self.max_simple_number_value = i64::MAX;
      self.max_simple_string_length = usize::MAX;
    }
    self
  }
}
