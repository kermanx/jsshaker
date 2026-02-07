use crate::value::PropertyKeyValue;

pub fn include_private_identifier_name(name: &str) -> String {
  format!("__#private__{}", name)
}

impl<'a> PropertyKeyValue<'a> {
  pub fn is_private_identifier(&self) -> bool {
    match self {
      PropertyKeyValue::String(s) => s.starts_with("__#private__"),
      PropertyKeyValue::Symbol(_) => false,
    }
  }
}
