#[derive(Debug, Default)]
pub struct FnCacheStats {
  // Overall metrics
  pub total_calls: usize,
  pub cache_attempts: usize,
  pub cache_hits: usize,
  pub cache_misses: usize,
  pub cache_updates: usize,

  // Miss reason breakdown
  pub miss_config_disabled: usize,
  pub miss_non_copyable_this: usize,
  pub miss_non_copyable_args: usize,
  pub miss_rest_params: usize,
  pub miss_non_copyable_return: usize,
  pub miss_state_untrackable: usize,
  pub miss_read_dep_incompatible: usize,
  pub miss_cache_empty: usize,

  // Per-function statistics (optional: implement later if needed)
  pub cache_table_size: usize,
}

impl FnCacheStats {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn print_summary(&self) {
    println!("\n=== Function Cache Statistics ===");
    println!("Total Function Calls:  {}", self.total_calls);
    println!("Cache Key Generated:   {}", self.cache_attempts);
    println!("Cache Hits:            {} ({:.1}%)", self.cache_hits, self.hit_rate_percent());
    println!(
      "Cache Misses:          {} ({:.1}%)",
      self.cache_misses,
      100.0 - self.hit_rate_percent()
    );
    println!("Successful Updates:    {}", self.cache_updates);
    println!("Cache Table Size:      {} entries", self.cache_table_size);

    if self.cache_misses > 0 {
      println!("\n--- Miss Reason Breakdown ---");
      self.print_miss_reason("Config Disabled", self.miss_config_disabled);
      self.print_miss_reason("Non-copyable This", self.miss_non_copyable_this);
      self.print_miss_reason("Non-copyable Args", self.miss_non_copyable_args);
      self.print_miss_reason("Rest Parameters", self.miss_rest_params);
      self.print_miss_reason("Non-copyable Return", self.miss_non_copyable_return);
      self.print_miss_reason("State Untrackable", self.miss_state_untrackable);
      self.print_miss_reason("Read Dep Incompatible", self.miss_read_dep_incompatible);
      self.print_miss_reason("Cache Empty (First Call)", self.miss_cache_empty);
    }
    println!("=================================\n");
  }

  fn hit_rate_percent(&self) -> f64 {
    if self.cache_attempts == 0 {
      0.0
    } else {
      (self.cache_hits as f64 / self.cache_attempts as f64) * 100.0
    }
  }

  fn print_miss_reason(&self, reason: &str, count: usize) {
    if count > 0 {
      let total_calls = self.total_calls.max(1);
      println!(
        "  {:30} {:8} ({:5.1}% of total calls)",
        format!("{}:", reason),
        count,
        (count as f64 / total_calls as f64) * 100.0
      );
    }
  }
}
