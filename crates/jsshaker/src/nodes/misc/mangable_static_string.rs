use crate::{analyzer::Analyzer, dep::DepAtom, entity::Entity, transformer::Transformer};

impl<'a> Analyzer<'a> {
  pub fn exec_mangable_static_string(
    &mut self,
    node: impl Into<DepAtom>,
    str: &'a str,
  ) -> Entity<'a> {
    let atom = self.mangler.use_node_atom(node);
    self.factory.string(str, atom)
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_mangable_static_string(
    &self,
    key: impl Into<DepAtom>,
    original: &'a str,
  ) -> &'a str {
    let mut mangler = self.mangler.borrow_mut();
    mangler.resolve_node(key).unwrap_or(original)
  }
}
