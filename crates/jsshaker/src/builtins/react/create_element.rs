use crate::{analyzer::Factory, builtin_string, entity::Entity, value::ObjectPrototype};

pub fn create_react_create_element_impl<'a>(factory: &'a Factory<'a>) -> Entity<'a> {
  factory.implemented_builtin_fn("React::createElement", |analyzer, dep, _this, args| {
    let tag = args.get(analyzer, 0);
    let props = args.get(analyzer, 1);
    let props = match props.test_nullish() {
      Some(true) => analyzer.factory.computed(
        analyzer
          .new_empty_object(ObjectPrototype::Builtin(&analyzer.builtins.prototypes.object), None)
          .into(),
        props,
      ),
      Some(false) => props,
      None => analyzer.factory.union((
        props,
        analyzer.factory.computed(
          analyzer
            .new_empty_object(ObjectPrototype::Builtin(&analyzer.builtins.prototypes.object), None)
            .into(),
          props,
        ),
      )),
    };

    let children = analyzer.new_empty_array();
    for element in args.elements.iter().skip(2) {
      children.push_element(*element);
    }
    if let Some(rest) = args.rest {
      children.init_rest(rest);
    }

    // Special prop: ref
    let r#ref = props.get_property(analyzer, analyzer.factory.no_dep, builtin_string!("ref"));
    if r#ref.test_nullish() != Some(true) {
      // TODO: currently we haven't implemented useRef, so we just consider it as a callback
      analyzer.exec_escaped_fn("React_ref", move |analyzer| {
        r#ref.call(
          analyzer,
          analyzer.factory.no_dep,
          analyzer.factory.unknown,
          analyzer.factory.unknown_arguments,
        )
      });
    }

    // Special prop: key
    let key = props.get_property(analyzer, analyzer.factory.no_dep, builtin_string!("key"));
    if r#ref.test_nullish() != Some(true) {
      analyzer.consume(key);
    }

    // FIXME: Should create new object
    props.set_property(
      analyzer,
      analyzer.factory.no_dep,
      builtin_string!("children"),
      children.into(),
    );

    let element = analyzer.factory.react_element(tag, analyzer.factory.computed(props, dep));
    analyzer.factory.computed(element, dep)
  })
}
