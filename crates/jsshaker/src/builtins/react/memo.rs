use crate::{analyzer::Factory, entity::Entity};

pub fn create_react_memo_impl<'a>(factory: &'a Factory<'a>) -> Entity<'a> {
  factory.implemented_builtin_fn("React::memo", |analyzer, dep, _this, args| {
    args.destruct_as_array(analyzer, dep, 1, false).0[0]
  })
}
