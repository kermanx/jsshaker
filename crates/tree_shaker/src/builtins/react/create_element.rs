use crate::entity::{Entity, EntityFactory, ObjectPrototype};

pub fn create_react_create_element_impl<'a>(factory: &'a EntityFactory<'a>) -> Entity<'a> {
  factory.implemented_builtin_fn("React::createElement", |analyzer, dep, _this, args| {
    let (args, children, _) = args.destruct_as_array(analyzer, dep, 2, true);
    let [tag, props] = args[..] else { unreachable!() };
    let props = match props.test_nullish() {
      Some(true) => analyzer
        .new_empty_object(ObjectPrototype::Builtin(&analyzer.builtins.prototypes.object), None)
        .into(),
      Some(false) => props,
      None => analyzer.factory.union((
        props,
        analyzer
          .new_empty_object(ObjectPrototype::Builtin(&analyzer.builtins.prototypes.object), None)
          .into(),
      )),
    };

    // Special prop: ref
    let r#ref = props.get_property(
      analyzer,
      analyzer.factory.empty_consumable,
      analyzer.factory.string("ref"),
    );
    if r#ref.test_nullish() != Some(true) {
      // TODO: currently we haven't implemented useRef, so we just consider it as a callback
      analyzer.exec_consumed_fn("React_ref", move |analyzer| {
        r#ref.call(
          analyzer,
          analyzer.factory.empty_consumable,
          analyzer.factory.unknown(),
          analyzer.factory.unknown(),
        )
      });
    }

    // Special prop: key
    let key = props.get_property(
      analyzer,
      analyzer.factory.empty_consumable,
      analyzer.factory.string("key"),
    );
    if r#ref.test_nullish() != Some(true) {
      analyzer.consume(key);
    }

    props.set_property(
      analyzer,
      analyzer.factory.empty_consumable,
      analyzer.factory.string("children"),
      children.unwrap(),
    );
    analyzer.factory.react_element(tag, props)
  })
}
