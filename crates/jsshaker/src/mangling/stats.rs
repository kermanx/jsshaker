use crate::transformer::Transformer;

#[derive(Debug, Default, Clone)]
pub struct ManglingStats {
  pub dynamic: usize,
  pub static_all: usize,
  pub static_mangled: usize,
}

impl<'a> Transformer<'a> {
  pub fn record_dynamic_property_key(&self) {
    if let Some(stats) = self.mangling_stats.as_ref() {
      let mut stats = stats.borrow_mut();
      stats.dynamic += 1;
    }
  }

  pub fn record_static_property_key(&self) {
    if let Some(stats) = self.mangling_stats.as_ref() {
      let mut stats = stats.borrow_mut();
      stats.static_all += 1;
    }
  }

  pub fn record_mangled_static_property_key(&self) {
    if let Some(stats) = self.mangling_stats.as_ref() {
      let mut stats = stats.borrow_mut();
      stats.static_mangled += 1;
    }
  }
}
