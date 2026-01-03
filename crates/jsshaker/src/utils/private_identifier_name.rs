use crate::{analyzer::Analyzer, value::PropertyKeyValue};

impl<'a> Analyzer<'a> {
  pub fn escape_private_identifier_name(&self, name: &str) -> &'a str {
    self.allocator.alloc_str(&format!("__#private__{}", name))
  }
}

impl<'a> PropertyKeyValue<'a> {
  pub fn is_private_identifier(&self) -> bool {
    match self {
      PropertyKeyValue::String(s) => s.starts_with("__#private__"),
      PropertyKeyValue::Symbol(_) => false,
    }
  }
}
