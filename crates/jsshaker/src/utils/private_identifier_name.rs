use oxc::ast::ast::{Class, PrivateIdentifier, Program, PropertyKey};
use oxc_ast_visit::{Visit, walk};
use rustc_hash::FxHashMap;

use crate::value::PropertyKeyValue;

const PRIVATE_IDENTIFIER_PREFIX: &str = "__#private__";

pub fn unescape_private_identifier_name(name: &str) -> &str {
  name
    .strip_prefix(PRIVATE_IDENTIFIER_PREFIX)
    .and_then(|stripped| stripped.split_once('#'))
    .map_or(name, |(_, name)| name)
}

impl<'a> PropertyKeyValue<'a> {
  pub fn is_private_identifier(&self) -> bool {
    match self {
      PropertyKeyValue::String(s) => s.starts_with(PRIVATE_IDENTIFIER_PREFIX),
      PropertyKeyValue::Symbol(_) => false,
    }
  }
}

#[derive(Default)]
pub struct PrivateIdentifierRegistry<'a> {
  stack: Vec<(u32, Vec<&'a str>)>,
  next_class_id: u32,
  class_ids: FxHashMap<usize, u32>,
}

impl<'a> PrivateIdentifierRegistry<'a> {
  pub fn resolve(&mut self, program: &Program<'a>) {
    self.visit_program(program);
  }

  pub fn escaped_name(&self, node: &PrivateIdentifier<'a>) -> String {
    let class_id =
      self.class_ids.get(&std::ptr::from_ref(node).addr()).copied().unwrap_or(u32::MAX);
    format!("{}{}#{}", PRIVATE_IDENTIFIER_PREFIX, class_id, node.name)
  }
}

impl<'a> Visit<'a> for PrivateIdentifierRegistry<'a> {
  fn visit_class(&mut self, class: &Class<'a>) {
    let names = class
      .body
      .body
      .iter()
      .filter_map(|element| match element.property_key() {
        Some(PropertyKey::PrivateIdentifier(key)) => Some(key.name.as_str()),
        _ => None,
      })
      .collect();
    let class_id = self.next_class_id;
    self.next_class_id += 1;
    self.stack.push((class_id, names));
    walk::walk_class(self, class);
    self.stack.pop();
  }

  fn visit_private_identifier(&mut self, node: &PrivateIdentifier<'a>) {
    let declaring = self.stack.iter().rev().find(|(_, names)| names.contains(&node.name.as_str()));
    if let Some(&(class_id, _)) = declaring.or_else(|| self.stack.last()) {
      self.class_ids.insert(std::ptr::from_ref(node).addr(), class_id);
    }
  }
}
