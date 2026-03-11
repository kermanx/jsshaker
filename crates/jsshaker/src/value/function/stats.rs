use std::collections::HashMap;

use rustc_hash::FxHashSet;

use crate::dep::DepAtom;

#[derive(Debug, Default, Clone)]
pub struct FnStatsData {
  pub total_calls: usize,
  pub cache_attempts: usize,
  pub cache_hits: usize,
  pub cache_updates: usize,

  // Function counts
  pub function_declarations: FxHashSet<DepAtom>,
  pub function_instances: usize,

  // Miss reason breakdown
  pub miss_config_disabled: usize,
  pub miss_non_copyable_this: usize,
  pub miss_non_copyable_args: usize,
  pub miss_rest_params: usize,
  pub miss_non_copyable_return: usize,
  pub miss_state_untrackable: usize,
  pub miss_read_dep_incompatible: usize,
  pub miss_cache_empty: usize,
}

impl FnStatsData {
  pub fn hit_rate_percent(&self) -> f64 {
    if self.cache_attempts == 0 {
      0.0
    } else {
      (self.cache_hits as f64 / self.cache_attempts as f64) * 100.0
    }
  }
}

#[derive(Debug, Default)]
pub struct FnStats {
  // Overall metrics
  pub overall: FnStatsData,

  // Per-function statistics
  pub per_function: HashMap<String, FnStatsData>,

  pub cache_table_size: usize,
}

impl FnStats {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn get_or_create_fn_stats(&mut self, fn_name: &str) -> &mut FnStatsData {
    self.per_function.entry(fn_name.to_string()).or_default()
  }
}
