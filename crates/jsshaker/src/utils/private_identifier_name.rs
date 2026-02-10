use crate::value::PropertyKeyValue;

const PRIVATE_IDENTIFIER_PREFIX: &str = "__#private__";

pub fn escape_private_identifier_name(name: &str) -> String {
  format!("{}{}", PRIVATE_IDENTIFIER_PREFIX, name)
}

pub fn unescape_private_identifier_name(name: &str) -> &str {
  if let Some(stripped) = name.strip_prefix(PRIVATE_IDENTIFIER_PREFIX) { stripped } else { name }
}

impl<'a> PropertyKeyValue<'a> {
  pub fn is_private_identifier(&self) -> bool {
    match self {
      PropertyKeyValue::String(s) => s.starts_with(PRIVATE_IDENTIFIER_PREFIX),
      PropertyKeyValue::Symbol(_) => false,
    }
  }
}
