use std::cell::Cell;

use oxc::{allocator, span::GetSpan};

use super::{
  EnumeratedProperties, IteratedElements, ObjectPrototype, ObjectValue, TypeofResult, ValueTrait,
  consumed_object,
};
use crate::{
  analyzer::Analyzer,
  dep::Dep,
  entity::Entity,
  scope::VariableScopeId,
  utils::{CalleeInfo, CalleeNode},
};

#[derive(Debug)]
pub struct FunctionValue<'a> {
  body_consumed: Cell<bool>,
  pub callee: CalleeInfo<'a>,
  pub variable_scope_stack: allocator::Vec<'a, VariableScopeId>,
  pub finite_recursion: bool,
  pub statics: &'a ObjectValue<'a>,
  /// The `prototype` property. Not `__proto__`.
  pub prototype: &'a ObjectValue<'a>,
}

impl<'a> ValueTrait<'a> for FunctionValue<'a> {
  fn consume(&'a self, analyzer: &mut Analyzer<'a>) {
    self.consume_body(analyzer);
    self.statics.consume(analyzer);
    self.prototype.consume(analyzer);
  }

  fn unknown_mutate(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) {
    self.consume(analyzer);
    consumed_object::unknown_mutate(analyzer, dep);
  }

  fn get_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    key: Entity<'a>,
  ) -> Entity<'a> {
    self.statics.get_property(analyzer, self.forward_dep(dep, analyzer), key)
  }

  fn set_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    key: Entity<'a>,
    value: Entity<'a>,
  ) {
    // TODO: Support analyzing this kind of mutation
    if analyzer.op_strict_eq(key, analyzer.factory.string("prototype")).0 != Some(false) {
      return consumed_object::set_property(analyzer, dep, key, value);
    }

    self.statics.set_property(analyzer, self.forward_dep(dep, analyzer), key, value);
  }

  fn delete_property(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>, key: Entity<'a>) {
    self.statics.delete_property(analyzer, self.forward_dep(dep, analyzer), key);
  }

  fn enumerate_properties(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
  ) -> EnumeratedProperties<'a> {
    if analyzer.config.unknown_property_read_side_effects {
      self.consume(analyzer);
    }
    consumed_object::enumerate_properties(self, analyzer, dep)
  }

  fn call(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    this: Entity<'a>,
    args: Entity<'a>,
  ) -> Entity<'a> {
    if self.body_consumed.get() {
      return consumed_object::call(self, analyzer, dep, this, args);
    }

    if self.check_recursion(analyzer) {
      self.consume_body(analyzer);
      return consumed_object::call(self, analyzer, dep, this, args);
    }

    self.call_impl::<false>(analyzer, dep, this, args, false)
  }

  fn construct(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    args: Entity<'a>,
  ) -> Entity<'a> {
    if self.body_consumed.get() {
      return consumed_object::construct(self, analyzer, dep, args);
    }

    if self.check_recursion(analyzer) {
      self.consume_body(analyzer);
      return consumed_object::construct(self, analyzer, dep, args);
    }

    self.construct_impl(analyzer, dep, args, false)
  }

  fn jsx(&'a self, analyzer: &mut Analyzer<'a>, props: Entity<'a>) -> Entity<'a> {
    self.call(
      analyzer,
      analyzer.factory.no_dep,
      analyzer.factory.unknown,
      analyzer.factory.arguments(analyzer.factory.vec1((false, props))),
    )
  }

  fn r#await(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) -> Entity<'a> {
    consumed_object::r#await(analyzer, dep)
  }

  fn iterate(&'a self, analyzer: &mut Analyzer<'a>, dep: Dep<'a>) -> IteratedElements<'a> {
    self.consume(analyzer);
    consumed_object::iterate(analyzer, dep)
  }

  fn get_typeof(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.string("function")
  }

  fn get_to_string(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    consumed_object::get_to_string(analyzer)
  }

  fn get_to_numeric(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    consumed_object::get_to_numeric(analyzer)
  }

  fn get_to_boolean(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.boolean(true)
  }

  fn get_to_property_key(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.get_to_string(analyzer)
  }

  fn get_to_jsx_child(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.unknown
  }

  fn get_own_keys(&'a self, analyzer: &Analyzer<'a>) -> Option<Vec<(bool, Entity<'a>)>> {
    self.statics.get_own_keys(analyzer)
  }

  fn get_constructor_prototype(
    &'a self,
    _analyzer: &Analyzer<'a>,
    dep: Dep<'a>,
  ) -> Option<(Dep<'a>, ObjectPrototype<'a>, ObjectPrototype<'a>)> {
    Some((dep, ObjectPrototype::Custom(self.statics), ObjectPrototype::Custom(self.prototype)))
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
}

impl<'a> FunctionValue<'a> {
  fn check_recursion(&self, analyzer: &Analyzer<'a>) -> bool {
    if !self.finite_recursion {
      let mut recursion_depth = 0usize;
      for scope in analyzer.scoping.call.iter().rev() {
        if scope.callee.node == self.callee.node {
          recursion_depth += 1;
          if recursion_depth >= analyzer.config.max_recursion_depth {
            return true;
          }
        }
      }
    }
    false
  }

  pub fn call_impl<const IS_NEW: bool>(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    this: Entity<'a>,
    args: Entity<'a>,
    consume: bool,
  ) -> Entity<'a> {
    let call_dep = analyzer.dep((self.callee.into_node(), dep));
    let ret_val = match self.callee.node {
      CalleeNode::Function(node) => analyzer.call_function(
        self.into(),
        self.callee,
        call_dep,
        node,
        &self.variable_scope_stack,
        this,
        args,
        consume,
      ),
      CalleeNode::ArrowFunctionExpression(node) => analyzer.call_arrow_function_expression(
        self.callee,
        call_dep,
        node,
        &self.variable_scope_stack,
        args,
        consume,
      ),
      CalleeNode::ClassConstructor(node) => {
        // if !CTOR {
        analyzer.call_class_constructor(
          self.callee,
          call_dep,
          node,
          &self.variable_scope_stack,
          this,
          args,
          consume,
        )
        // } else {
        //   analyzer.throw_builtin_error("Cannot invoke class constructor without 'new'");
        //   analyzer.factory.unknown
        // }
      }
      _ => unreachable!(),
    };
    let ret_val = if IS_NEW {
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
    analyzer.factory.computed(ret_val, call_dep)
  }

  pub fn construct_impl(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Dep<'a>,
    args: Entity<'a>,
    consume: bool,
  ) -> Entity<'a> {
    let m = self.prototype.is_mangable().then(|| analyzer.new_object_mangling_group());
    let target = analyzer.new_empty_object(ObjectPrototype::Custom(self.prototype), m);
    self.call_impl::<true>(analyzer, dep, target.into(), args, consume)
  }

  pub fn consume_body(&'a self, analyzer: &mut Analyzer<'a>) {
    if self.body_consumed.replace(true) {
      return;
    }

    analyzer.consume(self.callee.into_node());

    #[cfg(feature = "flame")]
    let name = self.callee.debug_name;
    #[cfg(not(feature = "flame"))]
    let name = "";

    analyzer.exec_consumed_fn(name, move |analyzer| {
      self.call_impl::<false>(
        analyzer,
        analyzer.factory.no_dep,
        analyzer.factory.unknown,
        analyzer.factory.unknown,
        true,
      )
    });
  }

  fn forward_dep(&self, dep: Dep<'a>, analyzer: &Analyzer<'a>) -> Dep<'a> {
    analyzer.dep((dep, self.callee.into_node()))
  }
}

impl<'a> Analyzer<'a> {
  pub fn new_function(&mut self, node: CalleeNode<'a>) -> &'a FunctionValue<'a> {
    let (statics, prototype) = self.new_function_object(Some(node.into()));
    let function = self.factory.alloc(FunctionValue {
      body_consumed: Cell::new(false),
      callee: self.new_callee_info(node),
      variable_scope_stack: allocator::Vec::from_iter_in(
        self.scoping.variable.stack.iter().copied(),
        self.allocator,
      ),
      finite_recursion: self.has_finite_recursion_notation(node.span()),
      statics,
      prototype,
    });

    let mut created_in_self = false;
    for scope in self.scoping.call.iter().rev() {
      if scope.callee.node == node {
        created_in_self = true;
        break;
      }
    }

    if created_in_self {
      function.consume_body(self);
    }

    function
  }
}
