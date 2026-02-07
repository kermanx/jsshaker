use crate::{
  Analyzer,
  dep::{Dep, DepAtom},
  entity::Entity,
  utils::CalleeNode,
  value::{
    ArgumentsValue, FunctionValue, ObjectPrototype, TypeofResult,
    cache::{FnCache, FnCacheTrackDeps, FnCachedInput},
  },
};

#[derive(Debug, Clone, Copy)]
pub struct FnCallInfo<'a> {
  pub func: &'a FunctionValue<'a>,
  pub call_dep: Dep<'a>,
  pub call_id: DepAtom,
  pub cache_key: Option<FnCachedInput<'a>>,
  pub this: Entity<'a>,
  pub args: ArgumentsValue<'a>,
  pub include: bool,
}

impl<'a> FunctionValue<'a> {
  pub fn call_impl<const IS_CTOR: bool>(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    mut this: Entity<'a>,
    mut args: ArgumentsValue<'a>,
    include: bool,
  ) -> Entity<'a> {
    let fn_name = self.callee.debug_name;

    if let Some(stats) = &analyzer.fn_stats {
      let mut stats = stats.borrow_mut();
      stats.overall.total_calls += 1;
      stats.get_or_create_fn_stats(fn_name).total_calls += 1;
    }

    let call_id = DepAtom::from_counter();
    let call_dep = analyzer.dep((self.callee.into_node(), dep, call_id));

    let cache_key = FnCache::get_key::<IS_CTOR>(analyzer, this, args, fn_name);
    let track_deps = if let Some(cache_key) = cache_key {
      if let Some(cached_ret) =
        self.cache.retrieve(analyzer, &cache_key, call_dep, this, args, fn_name)
      {
        return cached_ret;
      } else {
        Some(FnCacheTrackDeps::wrap::<false>(analyzer, call_id, &mut this, &mut args))
      }
    } else {
      None
    };

    let info = FnCallInfo { func: self, call_id, call_dep, cache_key, this, args, include };

    let (ret_val, tracking_data) = match self.callee.node {
      CalleeNode::Function(node) => analyzer.call_function(node, info),
      CalleeNode::ArrowFunctionExpression(node) => {
        analyzer.call_arrow_function_expression(node, info)
      }
      CalleeNode::ClassConstructor(node) => {
        // if !CTOR {
        analyzer.call_class_constructor(node, info)
        // } else {
        //   analyzer.throw_builtin_error("Cannot invoke class constructor without 'new'");
        //   analyzer.factory.unknown
        // }
      }
      CalleeNode::BoundFunction(bound_fn) => {
        analyzer.call_bound_function::<IS_CTOR>(bound_fn, info)
      }
      _ => unreachable!(),
    };

    let ret_val = if IS_CTOR {
      let typeof_ret = ret_val.test_typeof();
      match (
        typeof_ret.intersects(TypeofResult::Object),
        typeof_ret.intersects(TypeofResult::_Primitive),
      ) {
        (true, true) => analyzer.factory.union((ret_val, this)),
        (true, false) => ret_val,
        (false, true) => this,
        (false, false) => analyzer.factory.never,
      }
    } else {
      ret_val
    };

    if let Some(cache_key) = cache_key {
      let has_global_effects = analyzer.is_included(call_id);
      self.cache.update(
        analyzer,
        cache_key,
        ret_val,
        track_deps.unwrap(),
        tracking_data,
        has_global_effects,
        fn_name,
      );
    }

    analyzer.factory.computed(ret_val, call_dep)
  }

  pub fn construct_impl(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    args: ArgumentsValue<'a>,
    include: bool,
  ) -> Entity<'a> {
    let prototype = self.get_prototype(analyzer, dep);
    let target = if let Some(p) = prototype.get_object() {
      let target = analyzer.new_empty_object(ObjectPrototype::Custom(p), p.mangling_group.get());
      target.add_extra_dep(prototype.get_shallow_dep(analyzer));
      target
    } else {
      analyzer.new_empty_object(ObjectPrototype::Unknown(analyzer.dep(prototype)), None)
    };
    self.call_impl::<true>(analyzer, dep, target.into(), args, include)
  }
}
