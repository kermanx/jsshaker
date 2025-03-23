use crate::{
  analyzer::Analyzer, entity::Entity, mangling::MangleAtom, transformer::Transformer,
  utils::dep_id::DepId,
};

impl<'a> Analyzer<'a> {
  pub fn exec_mangable_static_string(&mut self, key: impl Into<DepId>, str: &'a str) -> Entity<'a> {
    let atom = self.load_data::<Option<MangleAtom>>(key.into());
    self.factory.mangable_string(str, *atom.get_or_insert_with(|| self.mangler.new_atom()))
  }
}

impl<'a> Transformer<'a> {
  pub fn transform_mangable_static_string(
    &self,
    key: impl Into<DepId>,
    original: &'a str,
  ) -> &'a str {
    let atom = self.get_data::<Option<MangleAtom>>(key.into()).unwrap();
    let mut mangler = self.mangler.borrow_mut();
    mangler.resolve(atom).unwrap_or(original)
  }
}
