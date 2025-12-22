use crate::{
  Analyzer,
  dep::Dep,
  entity::Entity,
  utils::CalleeNode,
  value::{
    ArgumentsValue, FunctionValue, ObjectPrototype, TypeofResult,
    cache::{FnCache, FnCachedInput},
  },
};

#[derive(Debug, Clone, Copy)]
pub struct FnCallInfo<'a> {
  pub func: &'a FunctionValue<'a>,
  pub call_dep: Dep<'a>,
  pub cache_key: Option<FnCachedInput<'a>>,
  pub this: Entity<'a>,
  pub args: ArgumentsValue<'a>,
  pub consume: bool,
}

impl<'a> FunctionValue<'a> {
  pub fn call_impl<const IS_CTOR: bool>(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    this: Entity<'a>,
    args: ArgumentsValue<'a>,
    consume: bool,
  ) -> Entity<'a> {
    let call_dep = analyzer.dep((self.callee.into_node(), dep));

    let cache_key = FnCache::get_key::<IS_CTOR>(analyzer, this, args);
    if !consume && let Some(cache_key) = cache_key {
      let cache = self.cache.borrow();
      if let Some(cached_ret) = cache.retrieve(analyzer, &cache_key) {
        drop(cache);
        analyzer.global_effect();
        analyzer.consume((call_dep, this, args));
        return cached_ret;
      }
    }

    let info = FnCallInfo { func: self, call_dep, cache_key, this, args, consume };

    let (ret_val, cache_tracking) = match self.callee.node {
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
      // if cache_tracking.has_outer_deps {
      //   println!("Has outer deps: {}", self.callee.debug_name);
      // }
      self.cache.borrow_mut().update_cache(analyzer, cache_key, ret_val, cache_tracking);
    }

    analyzer.factory.computed(ret_val, call_dep)
  }

  pub fn construct_impl(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    args: ArgumentsValue<'a>,
    consume: bool,
  ) -> Entity<'a> {
    let target = analyzer.new_empty_object(
      ObjectPrototype::Custom(self.prototype),
      self.prototype.mangling_group.get(),
    );
    self.call_impl::<true>(analyzer, dep, target.into(), args, consume)
  }
}
