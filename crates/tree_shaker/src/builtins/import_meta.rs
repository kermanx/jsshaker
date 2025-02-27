use super::{constants::IMPORT_META_OBJECT_ID, prototypes::BuiltinPrototypes, Builtins};
use crate::entity::{Entity, EntityFactory, ObjectProperty, ObjectPropertyValue};

impl<'a> Builtins<'a> {
  pub fn create_import_meta(
    factory: &'a EntityFactory<'a>,
    prototypes: &'a BuiltinPrototypes<'a>,
  ) -> Entity<'a> {
    let object = factory.builtin_object(IMPORT_META_OBJECT_ID, &prototypes.null, true);
    object.init_rest(ObjectPropertyValue::Property(
      Some(factory.immutable_unknown),
      Some(factory.immutable_unknown),
    ));

    // import.meta.url
    object.string_keyed.borrow_mut().insert(
      "url",
      ObjectProperty {
        definite: true,
        possible_values: vec![ObjectPropertyValue::Property(
          Some(factory.implemented_builtin_fn("import.meta.url", |analyzer, _, _, _| {
            analyzer.factory.unknown_string
          })),
          None,
        )],
        non_existent: Default::default(),
        key: None,
        mangling: None,
      },
    );

    object
  }
}
