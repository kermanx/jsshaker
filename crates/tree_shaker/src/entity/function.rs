use super::{
  consumed_object, Entity, EntityTrait, EnumeratedProperties, IteratedElements, ObjectEntity,
  TypeofResult,
};
use crate::{
  analyzer::Analyzer,
  consumable::Consumable,
  scope::VariableScopeId,
  use_consumed_flag,
  utils::{CalleeInfo, CalleeNode},
};
use oxc::span::GetSpan;
use std::{cell::Cell, rc::Rc};

#[derive(Debug)]
pub struct FunctionEntity<'a> {
  consumed: Rc<Cell<bool>>,
  body_consumed: Rc<Cell<bool>>,
  pub callee: CalleeInfo<'a>,
  pub variable_scope_stack: Rc<Vec<VariableScopeId>>,
  pub finite_recursion: bool,
  pub object: &'a ObjectEntity<'a>,
}

impl<'a> EntityTrait<'a> for FunctionEntity<'a> {
  fn consume(&'a self, analyzer: &mut Analyzer<'a>) {
    use_consumed_flag!(self);

    self.consume_body(analyzer);

    self.object.consume(analyzer);
  }

  fn unknown_mutate(&'a self, analyzer: &mut Analyzer<'a>, dep: Consumable<'a>) {
    self.consume(analyzer);
    consumed_object::unknown_mutate(analyzer, dep);
  }

  fn get_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
    key: Entity<'a>,
  ) -> Entity<'a> {
    self.object.get_property(analyzer, self.forward_dep(dep, analyzer), key)
  }

  fn set_property(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
    key: Entity<'a>,
    value: Entity<'a>,
  ) {
    self.object.set_property(analyzer, self.forward_dep(dep, analyzer), key, value);
  }

  fn delete_property(&'a self, analyzer: &mut Analyzer<'a>, dep: Consumable<'a>, key: Entity<'a>) {
    self.object.delete_property(analyzer, self.forward_dep(dep, analyzer), key);
  }

  fn enumerate_properties(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
  ) -> EnumeratedProperties<'a> {
    if analyzer.config.unknown_property_read_side_effects {
      self.consume(analyzer);
    }
    consumed_object::enumerate_properties(self, analyzer, dep)
  }

  fn call(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
    this: Entity<'a>,
    args: Entity<'a>,
  ) -> Entity<'a> {
    if self.consumed.get() {
      return consumed_object::call(self, analyzer, dep, this, args);
    }

    if !self.finite_recursion {
      let mut recursion_depth = 0usize;
      for scope in analyzer.scoping.call.iter().rev() {
        if scope.callee.node == self.callee.node {
          recursion_depth += 1;
          if recursion_depth >= analyzer.config.max_recursion_depth {
            self.consume_body(analyzer);
            return consumed_object::call(self, analyzer, dep, this, args);
          }
        }
      }
    }

    self.call_impl(analyzer, dep, this, args, false)
  }

  fn construct(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
    args: Entity<'a>,
  ) -> Entity<'a> {
    consumed_object::construct(self, analyzer, dep, args)
  }

  fn jsx(&'a self, analyzer: &mut Analyzer<'a>, props: Entity<'a>) -> Entity<'a> {
    self.call(
      analyzer,
      analyzer.factory.empty_consumable,
      analyzer.factory.immutable_unknown,
      analyzer.factory.arguments(vec![(false, props)]),
    )
  }

  fn r#await(&'a self, analyzer: &mut Analyzer<'a>, dep: Consumable<'a>) -> Entity<'a> {
    if self.consumed.get() {
      return consumed_object::r#await(analyzer, dep);
    }
    analyzer.factory.computed(self, dep)
  }

  fn iterate(&'a self, analyzer: &mut Analyzer<'a>, dep: Consumable<'a>) -> IteratedElements<'a> {
    self.consume(analyzer);
    consumed_object::iterate(analyzer, dep)
  }

  fn get_destructable(&'a self, analyzer: &Analyzer<'a>, dep: Consumable<'a>) -> Consumable<'a> {
    analyzer.consumable((self, dep))
  }

  fn get_typeof(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.string("function")
  }

  fn get_to_string(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    if self.consumed.get() {
      return consumed_object::get_to_string(analyzer);
    }
    analyzer.factory.computed_unknown_string(self)
  }

  fn get_to_numeric(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    if self.consumed.get() {
      return consumed_object::get_to_numeric(analyzer);
    }
    analyzer.factory.nan
  }

  fn get_to_boolean(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    analyzer.factory.boolean(true)
  }

  fn get_to_property_key(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    self.get_to_string(analyzer)
  }

  fn get_to_jsx_child(&'a self, analyzer: &Analyzer<'a>) -> Entity<'a> {
    if self.consumed.get() {
      analyzer.factory.immutable_unknown
    } else {
      // TODO: analyzer.thrown_builtin_error("Functions are not valid JSX children");
      analyzer.factory.string("")
    }
  }

  fn get_own_keys(&'a self, analyzer: &Analyzer<'a>) -> Option<Vec<(bool, Entity<'a>)>> {
    self.object.get_own_keys(analyzer)
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

impl<'a> FunctionEntity<'a> {
  pub fn call_impl(
    &'a self,
    analyzer: &mut Analyzer<'a>,
    dep: Consumable<'a>,
    this: Entity<'a>,
    args: Entity<'a>,
    consume: bool,
  ) -> Entity<'a> {
    let call_dep = analyzer.consumable((self.callee.into_node(), dep));
    let variable_scopes = self.variable_scope_stack.clone();
    let ret_val = match self.callee.node {
      CalleeNode::Function(node) => analyzer.call_function(
        self,
        self.callee,
        call_dep,
        node,
        variable_scopes,
        this,
        args,
        consume,
      ),
      CalleeNode::ArrowFunctionExpression(node) => analyzer.call_arrow_function_expression(
        self.callee,
        call_dep,
        node,
        variable_scopes,
        args,
        consume,
      ),
      _ => unreachable!(),
    };
    analyzer.factory.computed(ret_val, call_dep)
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
      self.call_impl(
        analyzer,
        analyzer.factory.empty_consumable,
        analyzer.factory.unknown(),
        analyzer.factory.unknown(),
        true,
      )
    });
  }

  fn forward_dep(&self, dep: Consumable<'a>, analyzer: &Analyzer<'a>) -> Consumable<'a> {
    analyzer.consumable((dep, self.callee.into_node()))
  }
}

impl<'a> Analyzer<'a> {
  pub fn new_function(&mut self, node: CalleeNode<'a>) -> Entity<'a> {
    let function = self.factory.alloc(FunctionEntity {
      consumed: Rc::new(Cell::new(false)),
      body_consumed: Rc::new(Cell::new(false)),
      callee: self.new_callee_info(node),
      variable_scope_stack: Rc::new(self.scoping.variable.stack.clone()),
      finite_recursion: self.has_finite_recursion_notation(node.span()),
      object: self.new_function_object(),
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
      self.factory.unknown()
    } else {
      function
    }
  }
}
