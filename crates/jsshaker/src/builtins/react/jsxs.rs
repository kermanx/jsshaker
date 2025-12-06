use crate::{analyzer::Factory, entity::Entity};

pub fn create_react_jsxs_impl<'a>(factory: &'a Factory<'a>) -> Entity<'a> {
  factory.implemented_builtin_fn("React::jsxs", |analyzer, dep, _this, args| {
    let tag = args.get(analyzer, 0);
    let props = args.get(analyzer, 1);
    let key = args.get(analyzer, 2);
    analyzer.consume(props.get_shallow_dep(analyzer));
    props.set_property(analyzer, analyzer.factory.no_dep, analyzer.factory.string("key"), key);
    let props = analyzer.factory.computed(props, dep);
    let element = analyzer.factory.react_element(tag, props);
    analyzer.factory.computed(element, dep)
  })
}
