use crate::{
  Analyzer, builtin_atom,
  dep::DepCollector,
  entity::Entity,
  value::{ObjectProperty, ObjectPropertyValue, ObjectPrototype, PropertyKeyValue},
};

impl<'a> Analyzer<'a> {
  pub fn create_import_meta(&mut self) -> Entity<'a> {
    let object = self.new_empty_object(ObjectPrototype::ImplicitOrNull, None);
    object.init_rest(
      self.factory,
      ObjectPropertyValue::Property(Some(self.factory.unknown), Some(self.factory.unknown)),
    );

    // import.meta.url
    object.keyed.borrow_mut().insert(
      PropertyKeyValue::String(builtin_atom!("url")),
      ObjectProperty {
        definite: true,
        enumerable: true,
        possible_values: self.factory.vec1(ObjectPropertyValue::Property(
          Some(self.factory.implemented_builtin_fn("import.meta.url", |analyzer, _, _, _| {
            analyzer.factory.unknown_string
          })),
          None,
        )),
        non_existent: DepCollector::new(self.factory.vec()),
        key: None,
        mangling: None,
      },
    );

    object.into()
  }
}
