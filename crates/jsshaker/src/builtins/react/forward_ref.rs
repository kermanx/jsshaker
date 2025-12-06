use crate::{analyzer::Factory, entity::Entity};

pub fn create_react_forward_ref_impl<'a>(factory: &'a Factory<'a>) -> Entity<'a> {
  factory.implemented_builtin_fn("React::forwardRef", |analyzer, dep, _this, args| {
    let renderer = args.get(analyzer, 0);
    let result = analyzer.dynamic_implemented_builtin(
      "React::ForwardRefReturn",
      move |analyzer, dep, this, args| {
        let props = args.get(analyzer, 0);
        let r#ref = analyzer.factory.unknown;

        renderer.call(
          analyzer,
          dep,
          this,
          analyzer.factory.arguments(analyzer.factory.alloc([props, r#ref]), None),
        )
      },
    );
    analyzer.factory.computed(result, dep)
  })
}
