mod arguments;
pub mod bound;
mod builtin;
pub mod cache;
pub mod call;
pub mod stats;

use std::cell::Cell;

use oxc::span::GetSpan;

use super::{
  EnumeratedProperties, IteratedElements, ObjectPrototype, ObjectValue, TypeofResult, ValueTrait,
  cacheable::Cacheable, escaped,
};
use crate::{
  analyzer::Analyzer,
  builtin_string,
  dep::{Dep, DepAtom},
  entity::Entity,
  scope::VariableScopeId,
  utils::{CalleeInfo, CalleeNode},
  value::{
    UnionHint,
    cache::{FnCache, FnCacheTrackDeps},
  },
};
pub use arguments::*;
pub use builtin::*;
pub use stats::FnStats;

#[derive(Debug)]
pub struct FunctionValue<'a> {
  pub callee: CalleeInfo<'a>,
  pub lexical_scope: Option<VariableScopeId>,
  pub finite_recursion: bool,
  pub statics: &'a ObjectValue<'a>,

  body_included: Cell<Option<&'a FnCacheTrackDeps<'a>>>,

  cache: FnCache<'a>,
}

impl<'a> ValueTrait<'a> for FunctionValue<'a> {
  fn include(&'a self, analyzer: &mut Analyzer<'a>) {
    self.include_body(analyzer, analyzer.factory.unknown, analyzer.factory.unknown_arguments);
    self.statics.include(analyzer);
  }

  fn unknown_mutate(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) {
    self.include(analyzer);
    escaped::unknown_mutate(analyzer, dep);
  }

  fn get_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    key: Entity<'a>,
  ) -> Entity<'a> {
    self.statics.get_property(analyzer, self, dep, key)
  }

  fn set_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    key: Entity<'a>,
    value: Entity<'a>,
  ) {
    if self.callee.node.is_generator()
      && analyzer.op_strict_eq(key, builtin_string!("prototype"), true).0 == Some(true)
    {
      return escaped::set_property(analyzer, dep, key, value);
    }
    self.statics.set_property(analyzer, dep, key, value);
  }

  fn delete_property(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>, key: Entity<'a>) {
    self.statics.delete_property(analyzer, dep, key);
  }

  fn enumerate_properties(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
  ) -> EnumeratedProperties<'a> {
    self.statics.enumerate_properties(analyzer, self, dep)
  }

  fn call(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    this: Entity<'a>,
    args: ArgumentsValue<'a>,
  ) -> Entity<'a> {
    if let Some(track_deps) = self.body_included.get() {
      analyzer.global_effect();
      track_deps.assoc(analyzer, dep, this, &args);
      return analyzer.factory.unknown;
    }

    if self.check_recursion(analyzer) {
      analyzer.global_effect();
      analyzer.include(dep);
      return self.include_body(analyzer, this, args);
    }

    self.call_impl::<false>(analyzer, dep, this, args, false)
  }

  fn construct(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    args: ArgumentsValue<'a>,
  ) -> Entity<'a> {
    if let Some(track_deps) = self.body_included.get() {
      analyzer.global_effect();
      track_deps.assoc(analyzer, dep, analyzer.factory.unknown, &args);
      return analyzer.factory.unknown;
    }

    if self.check_recursion(analyzer) {
      analyzer.global_effect();
      analyzer.include(dep);
      return self.include_body(analyzer, analyzer.factory.unknown, args);
    }

    self.construct_impl(analyzer, dep, args, false)
  }

  fn jsx(&'a self, analyzer: &mut Analyzer<'a>, props: Entity<'a>) -> Entity<'a> {
    self.call(
      analyzer,
      analyzer.factory.no_dep,
      analyzer.factory.unknown,
      analyzer.factory.arguments(analyzer.factory.alloc([props]), None),
    )
  }

  fn r#await(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) -> Entity<'a> {
    escaped::r#await(analyzer, dep)
  }

  fn iterate(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) -> IteratedElements<'a> {
    self.include(analyzer);
    escaped::iterate(analyzer, dep)
  }

  fn coerce_string(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    escaped::coerce_string(analyzer)
  }

  fn coerce_number(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    escaped::coerce_numeric(analyzer)
  }

  fn coerce_boolean(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.boolean(true)
  }

  fn coerce_property_key(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.coerce_string(analyzer)
  }

  fn coerce_jsx_child(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.unknown
  }

  fn get_keys(
    &'a self,
    analyzer: &Analyzer<'a>,
    check_proto: bool,
  ) -> Option<Vec<(bool, Entity<'a>)>> {
    self.statics.get_keys(analyzer, check_proto)
  }

  fn get_constructor_prototype(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
  ) -> Option<(Dep<'a>, ObjectPrototype<'a>, ObjectPrototype<'a>)> {
    let prototype = self.get_prototype(analyzer, dep);
    Some((
      dep,
      ObjectPrototype::Custom(self.statics),
      if let Some(prototype) = prototype.as_object() {
        ObjectPrototype::Custom(prototype)
      } else {
        ObjectPrototype::Unknown(analyzer.factory.dep(prototype))
      },
    ))
  }

  fn as_object(&'a self) -> Option<&'a ObjectValue<'a>> {
    self.statics.as_object()
  }

  fn test_typeof(&self) -> TypeofResult {
    TypeofResult::Function
  }

  fn test_truthy(&self) -> Option<bool> {
    Some(true)
  }

  fn test_nullish(&self) -> Option<bool> {
    Some(false)
  }

  fn get_union_hint(&self) -> UnionHint {
    UnionHint::Object
  }

  fn as_cacheable(&self, _analyzer: &Analyzer<'a>) -> Option<Cacheable<'a>> {
    Some(Cacheable::Function(self.callee.instance_id))
  }
}

impl<'a> FunctionValue<'a> {
  fn get_prototype(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) -> Entity<'a> {
    self.statics.get_property(analyzer, self, dep, builtin_string!("prototype"))
  }

  fn check_recursion(&self, analyzer: &Analyzer<'a>) -> bool {
    if !self.finite_recursion {
      let mut recursion_depth = 0usize;
      for scope in analyzer.scoping.call.iter().rev() {
        if scope.callee.instance_id == self.callee.instance_id {
          recursion_depth += 1;
          if recursion_depth >= analyzer.config.max_recursion_depth {
            return true;
          }
        }
      }
    }
    false
  }

  pub fn include_body(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    mut this: Entity<'a>,
    mut args: ArgumentsValue<'a>,
  ) -> Entity<'a> {
    if self.body_included.get().is_some() {
      return analyzer.factory.unknown;
    }

    let track_deps =
      FnCacheTrackDeps::wrap::<true>(analyzer, DepAtom::placeholder(), &mut this, &mut args);
    self.body_included.set(Some(analyzer.allocator.alloc(track_deps)));

    analyzer.include(self.callee.into_node());

    #[cfg(feature = "flame")]
    let name = self.callee.debug_name;
    #[cfg(not(feature = "flame"))]
    let name = "";

    analyzer.exec_included_fn(name, move |analyzer| {
      self.call_impl::<false>(analyzer, analyzer.factory.no_dep, this, args, true)
    })
  }
}

impl<'a> Analyzer<'a> {
  pub fn new_function_with_prototype(
    &mut self,
    node: CalleeNode<'a>,
  ) -> (&'a FunctionValue<'a>, &'a ObjectValue<'a>) {
    let (statics, prototype) = self.new_function_object(Some(node.into()));
    let function = self.factory.alloc(FunctionValue {
      callee: self.new_callee_info(node),
      lexical_scope: self.scoping.variable.top(),
      finite_recursion: self.has_finite_recursion_notation(node.span()),
      statics,
      body_included: Cell::new(None),
      cache: FnCache::new_in(self.allocator),
    });

    let mut created_in_self = false;
    for scope in self.scoping.call.iter().rev() {
      if scope.callee.node == node {
        created_in_self = true;
        break;
      }
    }

    if created_in_self {
      function.include_body(self, self.factory.unknown, self.factory.unknown_arguments);
    }

    (function, prototype)
  }

  pub fn new_function(&mut self, node: CalleeNode<'a>) -> &'a FunctionValue<'a> {
    self.new_function_with_prototype(node).0
  }
}
