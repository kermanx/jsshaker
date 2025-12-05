use crate::{analyzer::Factory, entity::Entity};

pub fn create_react_memo_impl<'a>(factory: &'a Factory<'a>) -> Entity<'a> {
  factory.implemented_builtin_fn("React::memo", |analyzer, _dep, _this, args| args.get(analyzer, 0))
}
