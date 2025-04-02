use oxc::allocator;

use crate::{analyzer::Factory, entity::Entity};

pub fn create_react_forward_ref_impl<'a>(factory: &'a Factory<'a>) -> Entity<'a> {
  factory.implemented_builtin_fn("React::forwardRef", |analyzer, dep, _this, args| {
    let renderer = args.destruct_as_array(analyzer, dep, 1, false).0[0];

    analyzer.dynamic_implemented_builtin(
      "React::ForwardRefReturn",
      move |analyzer, dep, this, args| {
        let props = args.destruct_as_array(analyzer, analyzer.factory.no_dep, 1, false).0[0];
        let r#ref = analyzer.factory.unknown;

        renderer.call(
          analyzer,
          dep,
          this,
          analyzer.factory.arguments(allocator::Vec::from_array_in(
            [(false, props), (false, r#ref)],
            analyzer.allocator,
          )),
        )
      },
    )
  })
}
