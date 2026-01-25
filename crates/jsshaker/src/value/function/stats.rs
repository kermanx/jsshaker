use std::collections::HashMap;

#[derive(Debug, Default, Clone)]
pub struct FnStatsData {
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

  pub fn print_summary(&self) {
    println!("\n=== Function Cache Statistics ===");
    println!("Total Function Calls:  {}", self.overall.total_calls);
    println!("Cache Key Generated:   {}", self.overall.cache_attempts);
    println!(
      "Cache Hits:            {} ({:.1}%)",
      self.overall.cache_hits,
      self.overall.hit_rate_percent()
    );
    println!(
      "Cache Misses:          {} ({:.1}%)",
      self.overall.cache_misses,
      100.0 - self.overall.hit_rate_percent()
    );
    println!("Successful Updates:    {}", self.overall.cache_updates);
    println!("Cache Table Size:      {} entries", self.cache_table_size);

    if self.overall.cache_misses > 0 {
      println!("\n--- Miss Reason Breakdown ---");
      self.print_miss_reason("Config Disabled", self.overall.miss_config_disabled);
      self.print_miss_reason("Non-copyable This", self.overall.miss_non_copyable_this);
      self.print_miss_reason("Non-copyable Args", self.overall.miss_non_copyable_args);
      self.print_miss_reason("Rest Parameters", self.overall.miss_rest_params);
      self.print_miss_reason("Non-copyable Return", self.overall.miss_non_copyable_return);
      self.print_miss_reason("State Untrackable", self.overall.miss_state_untrackable);
      self.print_miss_reason("Read Dep Incompatible", self.overall.miss_read_dep_incompatible);
      self.print_miss_reason("Cache Empty (First Call)", self.overall.miss_cache_empty);
    }

    // Print top functions by call count
    if !self.per_function.is_empty() {
      println!("\n--- Top Functions by Call Count (>100 calls) ---");
      let mut functions: Vec<_> =
        self.per_function.iter().filter(|(_, stats)| stats.total_calls > 100).collect();
      functions.sort_by(|a, b| b.1.total_calls.cmp(&a.1.total_calls));

      for (name, stats) in &functions {
        let hit_rate = if stats.cache_attempts > 0 {
          format!("{:.1}%", stats.hit_rate_percent())
        } else {
          "N/A".to_string()
        };
        println!(
          "  {:50} calls:{:6}  attempts:{:6}  hits:{:6} ({:>6})",
          Self::truncate_name(name, 50),
          stats.total_calls,
          stats.cache_attempts,
          stats.cache_hits,
          hit_rate
        );
      }

      if self.per_function.len() > functions.len() {
        println!(
          "  ... and {} more functions with <100 calls",
          self.per_function.len() - functions.len()
        );
      }
    }

    println!("=================================\n");
  }

  fn print_miss_reason(&self, reason: &str, count: usize) {
    if count > 0 {
      let total_calls = self.overall.total_calls.max(1);
      println!(
        "  {:30} {:8} ({:5.1}% of total calls)",
        format!("{}:", reason),
        count,
        (count as f64 / total_calls as f64) * 100.0
      );
    }
  }

  fn truncate_name(name: &str, max_len: usize) -> String {
    if name.len() <= max_len { name.to_string() } else { format!("{}...", &name[..max_len - 3]) }
  }
}
