use std::{cell::RefCell, fmt};

use oxc::{
  allocator::{self, FromIn},
  semantic::SymbolId,
  span::Atom,
};

use super::cf_scope::CfScopeId;
use crate::{
  analyzer::Analyzer,
  ast::DeclarationKind,
  define_box_bump_idx,
  dep::{Dep, LazyDep},
  entity::Entity,
  module::NamedExport,
  scope::rw_tracking::{ReadWriteTarget, TrackReadCachable},
  utils::ast::AstKind2,
  value::{ArgumentsValue, cachable::Cachable},
};

define_box_bump_idx! {
  pub struct VariableScopeId;
}

pub type EntityOrTDZ<'a> = Option<Entity<'a>>; // None for TDZ

#[derive(Debug, Clone, Copy)]
pub struct Variable<'a> {
  pub kind: DeclarationKind,
  pub cf_scope: CfScopeId,
  pub exhausted: Option<LazyDep<'a, Dep<'a>>>,
  pub value: EntityOrTDZ<'a>,
  pub decl_node: AstKind2<'a>,
}

pub struct VariableScope<'a> {
  pub variables: allocator::HashMap<'a, SymbolId, &'a RefCell<Variable<'a>>>,
  pub this: Option<Entity<'a>>,
  pub arguments: Option<(ArgumentsValue<'a>, allocator::Vec<'a, SymbolId>)>,
  pub super_class: Option<Entity<'a>>,
}

impl fmt::Debug for VariableScope<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let mut map = f.debug_map();
    for (k, v) in self.variables.iter() {
      let v = v.borrow();
      map.entry(&k, &format!("{:?} {}", v.kind, v.value.is_some()));
    }
    map.finish()
  }
}

impl<'a> VariableScope<'a> {
  pub fn new_in(allocator: &'a allocator::Allocator) -> Self {
    VariableScope {
      variables: allocator::HashMap::new_in(allocator),
      this: None,
      arguments: None,
      super_class: None,
    }
  }

  pub fn new_in_with_this(allocator: &'a allocator::Allocator, this: Entity<'a>) -> Self {
    Self { this: Some(this), ..Self::new_in(allocator) }
  }
}

impl<'a> Analyzer<'a> {
  fn declare_on_scope(
    &mut self,
    id: VariableScopeId,
    kind: DeclarationKind,
    symbol: SymbolId,
    decl_node: AstKind2<'a>,
    fn_value: Option<Entity<'a>>,
  ) {
    if let Some(variable) = self.scoping.variable.get(id).variables.get(&symbol) {
      // Here we can't use kind.is_untracked() because this time we are declaring a variable
      let old_kind = variable.borrow().kind;

      if old_kind.is_untracked() {
        self.consume(decl_node);
        if let Some(val) = fn_value {
          val.consume(self)
        }
        return;
      }

      if old_kind.is_shadowable() && kind.is_redeclarable() {
        // Redeclaration is sometimes allowed
        // var x = 1; var x = 2;
        // function f(x) { var x }
        let mut variable = variable.borrow_mut();
        variable.kind = kind;
        // FIXME: Not sure if this is correct - how to handle the first declaration?
        variable.decl_node = decl_node;
        drop(variable);
        if let Some(new_val) = fn_value {
          self.write_on_scope(id, symbol, new_val);
        }
      } else {
        // Re-declaration
      }
    } else {
      let has_fn_value = fn_value.is_some();
      let exhausted = if let Some(exhausted_variables) = &self.exhausted_variables {
        !has_fn_value && exhausted_variables.contains(&(self.current_module(), symbol))
      } else {
        false
      };

      let variable = self.allocator.alloc(RefCell::new(Variable {
        kind,
        cf_scope: if kind.is_var() {
          self.cf_scope_id_of_call_scope()
        } else {
          self.scoping.cf.current_id()
        },
        exhausted: exhausted
          .then(|| self.factory.lazy_dep(self.factory.vec1(self.dep((fn_value, decl_node))))),
        value: if exhausted { Some(self.factory.unknown) } else { fn_value },
        decl_node,
      }));
      self.scoping.variable.get_mut(id).variables.insert(symbol, variable);
      if has_fn_value {
        self.request_exhaustive_callbacks(ReadWriteTarget::Variable(id, symbol));
      }
    }
  }

  fn init_on_scope(
    &mut self,
    id: VariableScopeId,
    symbol: SymbolId,
    value: Option<Entity<'a>>,
    init_node: AstKind2<'a>,
  ) {
    let variable = self.scoping.variable.get_mut(id).variables.get_mut(&symbol).unwrap();

    let mut variable = variable.borrow_mut();
    if variable.kind.is_redeclarable() {
      if let Some(value) = value {
        drop(variable);
        self.write_on_scope(id, symbol, self.factory.computed(value, init_node));
      } else {
        // Do nothing
      }
    } else if let Some(deps) = variable.exhausted {
      deps.push(self, self.dep((init_node, value)));
    } else {
      variable.value =
        Some(self.factory.computed(value.unwrap_or(self.factory.undefined), init_node));
      self.request_exhaustive_callbacks(ReadWriteTarget::Variable(id, symbol));
    }
  }

  /// None: not in this scope
  /// Some(None): in this scope, but TDZ
  /// Some(Some(val)): in this scope, and val is the value
  pub fn read_on_scope(
    &mut self,
    id: VariableScopeId,
    symbol: SymbolId,
  ) -> Option<EntityOrTDZ<'a>> {
    self.scoping.variable.get(id).variables.get(&symbol).copied().map(|variable| {
      let variable_ref = variable.borrow();
      let value = variable_ref.value.or_else(|| {
        variable_ref
          .kind
          .is_var()
          .then(|| self.factory.computed(self.factory.undefined, variable_ref.decl_node))
      });

      let value = if let Some(dep) = variable_ref.exhausted {
        drop(variable_ref);
        if let Some(value) = value {
          Some(self.factory.computed(value, dep))
        } else {
          self.consume(dep);
          None
        }
      } else {
        let target_cf_scope = variable_ref.cf_scope;
        let may_change = if let Some(value) = variable_ref.value {
          if variable_ref.kind.is_const() {
            false
          } else if variable_ref.kind.is_var() {
            true
          } else if value.as_cachable() == Some(Cachable::Unknown) {
            false
          } else {
            !self.is_readonly_symbol(symbol)
          }
        } else {
          true
        };
        drop(variable_ref);
        self.track_read(
          target_cf_scope,
          ReadWriteTarget::Variable(id, symbol),
          Some(if may_change {
            TrackReadCachable::Mutable(value)
          } else {
            TrackReadCachable::Immutable
          }),
        );
        value
      };

      if value.is_none() {
        // TDZ
        let variable_ref = variable.borrow();
        self.consume(variable_ref.decl_node);
        self.handle_tdz();
      }

      value
    })
  }

  fn write_on_scope(&mut self, id: VariableScopeId, symbol: SymbolId, new_val: Entity<'a>) -> bool {
    if let Some(variable) = self.scoping.variable.get(id).variables.get(&symbol).copied() {
      let kind = variable.borrow().kind;
      if kind.is_untracked() {
        self.consume(new_val);
      } else if kind.is_const() {
        self.throw_builtin_error("Cannot assign to const variable");
        self.consume(variable.borrow().decl_node);
        new_val.consume(self);
      } else {
        let variable_ref = variable.borrow();
        let target_cf_scope = self.find_first_different_cf_scope(variable_ref.cf_scope);
        let dep = (self.get_exec_dep(target_cf_scope), variable_ref.decl_node);

        if let Some(deps) = variable_ref.exhausted {
          deps.push(self, self.dep((dep, new_val)));
        } else {
          let old_val = variable_ref.value;
          let (should_consume, indeterminate) = if old_val.is_some() {
            // Normal write
            self.track_write(ReadWriteTarget::Variable(id, symbol), target_cf_scope)
          } else if variable_ref.kind.is_redeclarable() {
            // Write uninitialized `var`
            self.track_write(ReadWriteTarget::Variable(id, symbol), target_cf_scope)
          } else {
            // TDZ write
            self.handle_tdz();
            (true, false)
          };
          drop(variable_ref);

          let mut variable_ref = variable.borrow_mut();
          if should_consume {
            let module_id = self.current_module();
            if let Some(exhausted_variables) = &mut self.exhausted_variables {
              exhausted_variables.insert((module_id, symbol));
            }
            variable_ref.exhausted =
              Some(self.factory.lazy_dep(self.factory.vec1(self.dep((dep, new_val, old_val)))));
            variable_ref.value = Some(self.factory.unknown);
          } else {
            variable_ref.value = Some(self.factory.computed(
              if indeterminate {
                self.factory.union((old_val.unwrap_or(self.factory.undefined), new_val))
              } else {
                new_val
              },
              dep,
            ));
          };
          drop(variable_ref);

          self.request_exhaustive_callbacks(ReadWriteTarget::Variable(id, symbol));
        }
      }
      true
    } else {
      false
    }
  }

  pub fn consume_on_scope(&mut self, id: VariableScopeId, symbol: SymbolId) -> bool {
    if let Some(variable) = self.scoping.variable.get(id).variables.get(&symbol).copied() {
      let variable_ref = *variable.borrow();
      if let Some(dep) = variable_ref.exhausted {
        self.consume(dep);
      } else {
        self.consume(variable_ref.decl_node);
        self.consume(variable_ref.value);

        let mut variable_ref = variable.borrow_mut();
        variable_ref.exhausted = Some(self.factory.consumed_lazy_dep);
        variable_ref.value = Some(self.factory.unknown);
      }
      true
    } else {
      false
    }
  }

  fn mark_untracked_on_scope(&mut self, symbol: SymbolId) {
    let cf_scope_depth = self.call_scope().cf_scope_depth;
    let variable = self.allocator.alloc(RefCell::new(Variable {
      exhausted: Some(self.factory.consumed_lazy_dep),
      kind: DeclarationKind::UntrackedVar,
      cf_scope: self.scoping.cf.stack[cf_scope_depth],
      value: Some(self.factory.unknown),
      decl_node: AstKind2::Environment,
    }));
    let old = self.variable_scope_mut().variables.insert(symbol, variable);
    assert!(old.is_none());
  }

  pub fn consume_arguments_on_scope(&mut self, id: VariableScopeId) -> bool {
    if let Some((args_value, args_symbols)) = &mut self.scoping.variable.get_mut(id).arguments {
      let args_value = *args_value;
      let args_symbols = args_symbols.drain(..).collect::<Vec<_>>();
      self.consume(args_value);
      let mut arguments_consumed = true;
      for symbol in args_symbols {
        if !self.consume_on_scope(id, symbol) {
          // Still inside parameter declaration
          arguments_consumed = false;
        }
      }
      arguments_consumed
    } else {
      true
    }
  }
}

impl<'a> Analyzer<'a> {
  pub fn declare_symbol(
    &mut self,
    symbol: SymbolId,
    decl_node: AstKind2<'a>,
    exporting: bool,
    kind: DeclarationKind,
    fn_value: Option<Entity<'a>>,
  ) {
    let variable_scope = self.scoping.variable.current_id();
    self.declare_on_scope(variable_scope, kind, symbol, decl_node, fn_value);

    if exporting {
      let name = Atom::from_in(self.semantic().scoping().symbol_name(symbol), self.allocator);
      let dep = self.factory.no_dep;
      self
        .module_info_mut()
        .named_exports
        .insert(name, NamedExport::Variable(variable_scope, symbol, dep));
    }

    if kind == DeclarationKind::FunctionParameter
      && let Some(arguments) = &mut self.variable_scope_mut().arguments
    {
      arguments.1.push(symbol);
    }
  }

  pub fn init_symbol(
    &mut self,
    symbol: SymbolId,
    value: Option<Entity<'a>>,
    init_node: AstKind2<'a>,
  ) {
    let variable_scope = self.scoping.variable.current_id();
    self.init_on_scope(variable_scope, symbol, value, init_node);
  }

  /// `None` for TDZ
  pub fn read_symbol(&mut self, symbol: SymbolId) -> EntityOrTDZ<'a> {
    for depth in (0..self.scoping.variable.stack.len()).rev() {
      let id = self.scoping.variable.stack[depth];
      if let Some(value) = self.read_on_scope(id, symbol) {
        return value;
      }
    }
    self.mark_unresolved_reference(symbol);
    Some(self.factory.unknown)
  }

  pub fn write_symbol(&mut self, symbol: SymbolId, new_val: Entity<'a>) {
    for depth in (0..self.scoping.variable.stack.len()).rev() {
      let id = self.scoping.variable.stack[depth];
      if self.write_on_scope(id, symbol, new_val) {
        return;
      }
    }
    self.consume(new_val);
    self.mark_unresolved_reference(symbol);
  }

  fn mark_unresolved_reference(&mut self, symbol: SymbolId) {
    if self.semantic().scoping().symbol_flags(symbol).is_function_scoped_declaration() {
      self.mark_untracked_on_scope(symbol);
    } else {
      self.throw_builtin_error("Unresolved identifier reference");
    }
  }

  pub fn handle_tdz(&mut self) {
    self.throw_builtin_error("Cannot access variable before initialization");
    self.refer_to_global();
  }

  pub fn get_this(&self) -> Entity<'a> {
    for depth in (0..self.scoping.variable.stack.len()).rev() {
      let scope = self.scoping.variable.get_from_depth(depth);
      if let Some(this) = scope.this {
        return this;
      }
    }
    unreachable!()
  }

  pub fn get_super(&mut self) -> Entity<'a> {
    for depth in (0..self.scoping.variable.stack.len()).rev() {
      let scope = self.scoping.variable.get_from_depth(depth);
      if let Some(super_class) = scope.super_class {
        return super_class;
      }
    }
    self.throw_builtin_error("Unsupported reference to 'super'");
    self.factory.unknown
  }
}
