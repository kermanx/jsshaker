use super::dependencies::check_dependencies;
use crate::{
  entity::{Entity, EntityFactory},
  module::ModuleId,
};
use oxc::span::Span;
use rustc_hash::FxHashMap;

pub type ReactUseMemos<'a> = FxHashMap<(ModuleId, Span), Entity<'a>>;

pub fn create_react_use_memo_impl<'a>(factory: &'a EntityFactory<'a>) -> Entity<'a> {
  factory.implemented_builtin_fn("React::useMemo", |analyzer, dep, _this, args| {
    let [calculate, dependencies] = args.destruct_as_array(analyzer, dep, 2, false).0[..] else {
      unreachable!()
    };

    let (changed, dep) = check_dependencies(analyzer, dep, dependencies);

    let span = (analyzer.current_module(), analyzer.current_span());
    if changed {
      let result =
        calculate.call(analyzer, dep, analyzer.factory.unknown(), analyzer.factory.empty_arguments);
      analyzer.builtins.react_data.memos.insert(span, result);
      result
    } else {
      analyzer.factory.computed(analyzer.builtins.react_data.memos[&span], dep)
    }
  })
}
