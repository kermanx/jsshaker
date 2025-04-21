use super::{Builtins, constants::IMPORT_META_OBJECT_ID, prototypes::BuiltinPrototypes};
use crate::{
  analyzer::Factory,
  dep::DepCollector,
  entity::Entity,
  value::{ObjectProperty, ObjectPropertyValue, ObjectPrototype, PropertyKeyValue},
};

impl<'a> Builtins<'a> {
  pub fn create_import_meta(
    factory: &'a Factory<'a>,
    _prototypes: &'a BuiltinPrototypes<'a>,
  ) -> Entity<'a> {
    let object =
      factory.builtin_object(IMPORT_META_OBJECT_ID, ObjectPrototype::ImplicitOrNull, true);
    object.init_rest(
      factory,
      ObjectPropertyValue::Property(Some(factory.unknown), Some(factory.unknown)),
    );

    // import.meta.url
    object.keyed.borrow_mut().insert(
      PropertyKeyValue::String("url"),
      ObjectProperty {
        definite: true,
        enumerable: true,
        possible_values: factory.vec1(ObjectPropertyValue::Property(
          Some(factory.implemented_builtin_fn("import.meta.url", |analyzer, _, _, _| {
            analyzer.factory.unknown_string
          })),
          None,
        )),
        non_existent: DepCollector::new(factory.vec()),
        key: None,
        mangling: None,
      },
    );

    object.into()
  }
}
