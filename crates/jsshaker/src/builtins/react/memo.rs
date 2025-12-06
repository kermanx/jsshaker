use crate::{analyzer::Factory, entity::Entity};

pub fn create_react_memo_impl<'a>(factory: &'a Factory<'a>) -> Entity<'a> {
  factory.implemented_builtin_fn("React::memo", |analyzer, dep, _this, args| {
    let renderer = args.get(analyzer, 0);
    analyzer.factory.computed(renderer, dep)
  })
}
