use std::{cell::RefCell, fmt};

use oxc::{
  allocator::{self, FromIn},
  semantic::SymbolId,
  span::Atom,
};

use super::cf_scope::CfScopeId;
use crate::{
  analyzer::{
    Analyzer,
    rw_tracking::{ReadWriteTarget, TrackReadCachable},
  },
  ast::DeclarationKind,
  define_box_bump_idx,
  dep::{Dep, DepAtom, LazyDep},
  entity::Entity,
  module::ExportedValue,
  utils::ast::AstKind2,
  value::{ArgumentsValue, cacheable::Cacheable},
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
  fn variable(
    &self,
    scope: VariableScopeId,
    symbol: SymbolId,
  ) -> Option<&'a RefCell<Variable<'a>>> {
    self.scoping.variable.get(scope).variables.get(&symbol).copied()
  }

  fn declare_on_scope(
    &mut self,
    scope: VariableScopeId,
    kind: DeclarationKind,
    symbol: SymbolId,
    decl_node: AstKind2<'a>,
    fn_value: Option<Entity<'a>>,
  ) {
    if let Some(variable) = self.variable(scope, symbol) {
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
          self.write_on_scope(scope, symbol, new_val, false);
        }
      } else {
        // Re-declaration
      }
    } else {
      let has_fn_value = fn_value.is_some();
      let exhausted = if let Some(exhausted_variables) = &self.exhausted_variables {
        !has_fn_value && exhausted_variables.contains(&(self.current_module, symbol))
      } else {
        false
      };

      let variable = self.allocator.alloc(RefCell::new(Variable {
        kind,
        cf_scope: if kind.is_var() {
          let depth = self.call_scope().cf_scope_depth;
          self.scoping.cf.stack[depth]
        } else {
          self.scoping.cf.current_id()
        },
        exhausted: exhausted
          .then(|| self.factory.lazy_dep(self.factory.vec1(self.dep((fn_value, decl_node))))),
        value: if exhausted { Some(self.factory.unknown) } else { fn_value },
        decl_node,
      }));
      self.scoping.variable.get_mut(scope).variables.insert(symbol, variable);
      if has_fn_value {
        self.request_exhaustive_callbacks(ReadWriteTarget::Variable(scope, symbol));
      }
    }
  }

  fn init_on_scope(
    &mut self,
    scope: VariableScopeId,
    symbol: SymbolId,
    value: Option<Entity<'a>>,
    init_node: AstKind2<'a>,
  ) {
    let mut variable = self.variable(scope, symbol).unwrap().borrow_mut();
    if variable.kind.is_redeclarable() {
      if let Some(value) = value {
        drop(variable);
        self.write_on_scope(scope, symbol, self.factory.computed(value, init_node), false);
      } else {
        // Do nothing
      }
    } else if let Some(deps) = variable.exhausted {
      deps.push(self, self.dep((init_node, value)));
    } else {
      variable.value =
        Some(self.factory.computed(value.unwrap_or(self.factory.undefined), init_node));
      self.request_exhaustive_callbacks(ReadWriteTarget::Variable(scope, symbol));
    }
  }

  /// None: not in this scope
  /// Some(None): in this scope, but TDZ
  /// Some(Some(val)): in this scope, and val is the value
  pub fn read_on_scope(
    &mut self,
    scope: VariableScopeId,
    symbol: SymbolId,
  ) -> Option<EntityOrTDZ<'a>> {
    let variable = self.variable(scope, symbol)?.borrow();
    let decl_node = variable.decl_node;

    let value = variable.value.or_else(|| {
      variable.kind.is_var().then(|| self.factory.computed(self.factory.undefined, decl_node))
    });

    let value = if let Some(dep) = variable.exhausted {
      drop(variable);
      if let Some(value) = value {
        Some(self.factory.computed(value, dep))
      } else {
        self.consume(dep);
        None
      }
    } else {
      let cf_scope = variable.cf_scope;
      let may_change = if let Some(value) = variable.value {
        if variable.kind.is_const() {
          false
        } else if variable.kind.is_var() {
          true
        } else if value.as_cacheable() == Some(Cacheable::Unknown) {
          false
        } else {
          !self.is_readonly_symbol(symbol)
        }
      } else {
        true
      };
      drop(variable);
      self.track_read(
        cf_scope,
        ReadWriteTarget::Variable(scope, symbol),
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
      self.consume(decl_node);
      self.handle_tdz();
    }

    Some(value)
  }

  pub fn write_on_scope(
    &mut self,
    scope: VariableScopeId,
    symbol: SymbolId,
    new_val: Entity<'a>,
    force_indeterminate: bool,
  ) -> bool {
    let Some(variable_cell) = self.variable(scope, symbol) else {
      return false;
    };
    let variable = variable_cell.borrow();
    let kind = variable.kind;
    if kind.is_untracked() {
      self.consume(new_val);
    } else if kind.is_const() {
      self.throw_builtin_error("Cannot assign to const variable");
      self.consume(variable.decl_node);
      new_val.consume(self);
    } else {
      let target_cf_scope = self.find_first_different_cf_scope(variable.cf_scope);
      let dep = (self.get_exec_dep(target_cf_scope), variable.decl_node);

      if let Some(deps) = variable.exhausted {
        deps.push(self, self.dep((dep, new_val)));
      } else {
        let old_val = variable.value;
        let (should_consume, cf_indeterminate) = if old_val.is_some() {
          // Normal write
          self.track_write(target_cf_scope, ReadWriteTarget::Variable(scope, symbol), Some(new_val))
        } else if variable.kind.is_redeclarable() {
          // Write uninitialized `var`
          self.track_write(target_cf_scope, ReadWriteTarget::Variable(scope, symbol), Some(new_val))
        } else {
          // TDZ write
          self.handle_tdz();
          (true, false)
        };
        drop(variable);

        let mut variable = variable_cell.borrow_mut();
        if should_consume {
          let module_id = self.current_module;
          if let Some(exhausted_variables) = &mut self.exhausted_variables {
            exhausted_variables.insert((module_id, symbol));
          }
          variable.exhausted =
            Some(self.factory.lazy_dep(self.factory.vec1(self.dep((dep, new_val, old_val)))));
          variable.value = Some(self.factory.unknown);
        } else {
          variable.value = Some(self.factory.computed(
            if force_indeterminate || cf_indeterminate {
              self.factory.union((old_val.unwrap_or(self.factory.undefined), new_val))
            } else {
              new_val
            },
            dep,
          ));
        };
        drop(variable);

        self.request_exhaustive_callbacks(ReadWriteTarget::Variable(scope, symbol));
      }
    }
    true
  }

  pub fn consume_on_scope(&mut self, scope: VariableScopeId, symbol: SymbolId) -> bool {
    if let Some(variable_cell) = self.variable(scope, symbol) {
      let mut variable = variable_cell.borrow_mut();
      if let Some(dep) = variable.exhausted {
        self.consume(dep);
      } else {
        let old_variable = *variable;
        variable.exhausted = Some(self.factory.consumed_lazy_dep);
        variable.value = Some(self.factory.unknown);
        drop(variable);

        self.consume(old_variable.decl_node);
        self.consume(old_variable.value);
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

  pub fn consume_arguments_on_scope(&mut self, scope: VariableScopeId) -> bool {
    if let Some((args_value, args_symbols)) = &mut self.scoping.variable.get_mut(scope).arguments {
      let args_value = *args_value;
      let args_symbols = args_symbols.drain(..).collect::<Vec<_>>();
      self.consume(args_value);
      let mut arguments_consumed = true;
      for symbol in args_symbols {
        if !self.consume_on_scope(scope, symbol) {
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
    exporting: Option<DepAtom>,
    kind: DeclarationKind,
    fn_value: Option<Entity<'a>>,
  ) {
    let variable_scope = self.scoping.variable.top().unwrap();
    self.declare_on_scope(variable_scope, kind, symbol, decl_node, fn_value);

    if let Some(exporting) = exporting {
      let name = Atom::from_in(self.semantic().scoping().symbol_name(symbol), self.allocator);
      self.module_info_mut().named_exports.insert(
        name,
        if let Some(fn_value) = fn_value {
          ExportedValue::Function(fn_value, exporting)
        } else {
          ExportedValue::Variable(variable_scope, symbol, exporting)
        },
      );
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
    let variable_scope = self.scoping.variable.top().unwrap();
    self.init_on_scope(variable_scope, symbol, value, init_node);
  }

  /// `None` for TDZ
  pub fn read_symbol(&mut self, symbol: SymbolId) -> EntityOrTDZ<'a> {
    let mut scope = self.scoping.variable.top();
    while let Some(s) = scope {
      if let Some(value) = self.read_on_scope(s, symbol) {
        return value;
      }
      scope = self.scoping.variable.get_parent(s);
    }
    self.mark_unresolved_reference(symbol);
    Some(self.factory.unknown)
  }

  pub fn write_symbol(&mut self, symbol: SymbolId, new_val: Entity<'a>) {
    let mut scope = self.scoping.variable.top();
    while let Some(s) = scope {
      if self.write_on_scope(s, symbol, new_val, false) {
        return;
      }
      scope = self.scoping.variable.get_parent(s);
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
    self.global_effect();
  }

  pub fn get_this(&self) -> Entity<'a> {
    for scope in self.scoping.variable.iter_rev() {
      if let Some(this) = scope.this {
        return this;
      }
    }
    unreachable!()
  }

  pub fn get_super(&mut self) -> Entity<'a> {
    for scope in self.scoping.variable.iter_rev() {
      if let Some(super_class) = scope.super_class {
        return super_class;
      }
    }
    self.throw_builtin_error("Unsupported reference to 'super'");
    self.factory.unknown
  }
}
