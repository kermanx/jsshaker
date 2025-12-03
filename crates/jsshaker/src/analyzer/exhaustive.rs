use std::{
  hash::{Hash, Hasher},
  mem,
  rc::Rc,
};

use oxc::semantic::SymbolId;
use rustc_hash::FxHashSet;

use crate::{
  analyzer::Analyzer,
  entity::Entity,
  scope::{CfScopeId, CfScopeKind, VariableScopeId, cf_scope::ReferredState},
  utils::flame,
  value::{ObjectId, PropertyKeyValue},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExhaustiveDepId<'a> {
  Variable(VariableScopeId, SymbolId),
  ObjectAll(ObjectId),
  ObjectField(ObjectId, PropertyKeyValue<'a>),
  __Object(ObjectId),
}

impl<'a> ExhaustiveDepId<'a> {
  fn object_read_extra(self) -> Option<ExhaustiveDepId<'a>> {
    match self {
      ExhaustiveDepId::ObjectField(id, _) => Some(ExhaustiveDepId::__Object(id)),
      _ => None,
    }
  }

  fn object_write_extra(self) -> Option<ExhaustiveDepId<'a>> {
    match self {
      ExhaustiveDepId::ObjectAll(id) => Some(ExhaustiveDepId::__Object(id)),
      ExhaustiveDepId::ObjectField(id, _) => Some(ExhaustiveDepId::ObjectAll(id)),
      _ => None,
    }
  }
}

#[derive(Debug)]
pub struct ExhaustiveData<'a> {
  pub clean: bool,
  pub temp_deps: Option<FxHashSet<ExhaustiveDepId<'a>>>,
  pub register_deps: Option<FxHashSet<ExhaustiveDepId<'a>>>,
}

#[derive(Clone)]
pub struct ExhaustiveCallback<'a> {
  pub handler: Rc<dyn Fn(&mut Analyzer<'a>) + 'a>,
  pub drain: bool,
}
impl PartialEq for ExhaustiveCallback<'_> {
  fn eq(&self, other: &Self) -> bool {
    self.drain == other.drain && Rc::ptr_eq(&self.handler, &other.handler)
  }
}
impl Eq for ExhaustiveCallback<'_> {}
impl Hash for ExhaustiveCallback<'_> {
  fn hash<H: Hasher>(&self, state: &mut H) {
    Rc::as_ptr(&self.handler).hash(state);
  }
}

impl<'a> Analyzer<'a> {
  pub fn exec_loop(&mut self, runner: impl Fn(&mut Analyzer<'a>) + 'a) {
    let runner = Rc::new(runner);

    self.exec_exhaustively("loop", true, false, runner.clone());

    let cf_scope = self.cf_scope();
    if cf_scope.referred_state != ReferredState::ReferredClean && cf_scope.deps.may_not_referred() {
      runner(self);
    }
  }

  pub fn exec_consumed_fn(
    &mut self,
    kind: &str,
    runner: impl Fn(&mut Analyzer<'a>) -> Entity<'a> + 'a,
  ) {
    let runner: Rc<dyn Fn(&mut Analyzer<'a>) + 'a> = Rc::new(move |analyzer| {
      let ret_val = runner(analyzer);
      if !analyzer.is_inside_pure() {
        analyzer.consume(ret_val);
      }
    });
    self.exec_exhaustively(kind, true, true, runner);
  }

  pub fn exec_async_or_generator_fn(&mut self, runner: impl Fn(&mut Analyzer<'a>) + 'a) {
    self.exec_exhaustively("async/generator", false, true, Rc::new(runner));
  }

  fn exec_exhaustively(
    &mut self,
    _kind: &str,
    drain: bool,
    register: bool,
    runner: Rc<dyn Fn(&mut Analyzer<'a>) + 'a>,
  ) {
    self.push_cf_scope(
      CfScopeKind::Exhaustive(ExhaustiveData {
        clean: true,
        temp_deps: drain.then(FxHashSet::default),
        register_deps: register.then(Default::default),
      }),
      true,
    );
    let mut round_counter = 0;
    loop {
      self.cf_scope_mut().exited = None;
      #[cfg(feature = "flame")]
      let _scope_guard = flame::start_guard(format!(
        "!{_kind}@{:06X} x{}",
        (Rc::as_ptr(&runner) as *const () as usize) & 0xFFFFFF,
        round_counter
      ));
      runner(self);
      round_counter += 1;
      if round_counter > 1000 {
        unreachable!("Exhaustive loop is too deep");
      }
      if !self.cf_scope_mut().post_exhaustive_iterate() {
        break;
      }
    }
    let id = self.pop_cf_scope();
    let data = self.scoping.cf.get_mut(id).exhaustive_data_mut().unwrap();
    if let Some(register_deps) = data.register_deps.take() {
      self.register_exhaustive_callbacks(drain, runner, register_deps);
    }
  }

  fn register_exhaustive_callbacks(
    &mut self,
    drain: bool,
    handler: Rc<dyn Fn(&mut Analyzer<'a>) + 'a>,
    deps: FxHashSet<ExhaustiveDepId<'a>>,
  ) {
    for id in deps {
      self
        .exhaustive_callbacks
        .entry(id)
        .or_default()
        .insert(ExhaustiveCallback { handler: handler.clone(), drain });
    }
  }

  pub fn mark_exhaustive_read(&mut self, id: ExhaustiveDepId<'a>, target: CfScopeId) {
    let target_depth = self.find_first_different_cf_scope(target);
    let mut registered = false;
    for depth in (target_depth..self.scoping.cf.stack.len()).rev() {
      let scope = self.scoping.cf.get_mut_from_depth(depth);
      if let Some(data) = scope.exhaustive_data_mut() {
        if data.clean
          && let Some(temp_deps) = data.temp_deps.as_mut()
        {
          temp_deps.insert(id);
          id.object_read_extra().map(|id| temp_deps.insert(id));
        }
        if !registered && let Some(register_deps) = data.register_deps.as_mut() {
          registered = true;
          register_deps.insert(id);
          id.object_read_extra().map(|id| register_deps.insert(id));
        }
      }
    }
  }

  pub fn mark_exhaustive_write(&mut self, id: ExhaustiveDepId, target: usize) -> (bool, bool) {
    let mut exhaustive = false;
    let mut indeterminate = false;
    let mut need_mark = true;
    for depth in target..self.scoping.cf.stack.len() {
      let scope = self.scoping.cf.get_mut_from_depth(depth);
      indeterminate |= scope.is_indeterminate();
      if let Some(data) = scope.exhaustive_data_mut() {
        exhaustive = true;
        if (need_mark || data.register_deps.is_some())
          && data.clean
          && let Some(temp_deps) = &data.temp_deps
        {
          if temp_deps.contains(&id) {
            data.clean = false;
          } else if let Some(id) = id.object_write_extra()
            && temp_deps.contains(&id)
          {
            data.clean = false;
          }
          need_mark = false;
        }
      }
    }
    (exhaustive, indeterminate)
  }

  pub fn request_exhaustive_callbacks(&mut self, id: ExhaustiveDepId<'a>) -> bool {
    let mut found = false;
    let mut do_request = |id: ExhaustiveDepId<'a>| {
      if let Some(runners) = self.exhaustive_callbacks.get_mut(&id)
        && !runners.is_empty()
      {
        self.pending_deps.extend(runners.drain());
        found = true;
      }
    };
    do_request(id);
    id.object_write_extra().map(do_request);
    found
  }

  pub fn call_exhaustive_callbacks(&mut self) -> bool {
    if self.pending_deps.is_empty() {
      return false;
    }
    let old_try_catch_depth = self.scoping.try_catch_depth.take();
    loop {
      let runners = mem::take(&mut self.pending_deps);
      for runner in runners {
        // let old_count = self.referred_deps.debug_count();
        let ExhaustiveCallback { handler: runner, drain } = runner;
        self.exec_exhaustively("dep", drain, true, runner.clone());
        // let new_count = self.referred_deps.debug_count();
        // self.debug += 1;
      }
      if self.pending_deps.is_empty() {
        self.scoping.try_catch_depth = old_try_catch_depth;
        return true;
      }
    }
  }

  pub fn has_exhaustive_scope_since(&self, target_depth: usize) -> bool {
    self.scoping.cf.iter_stack().skip(target_depth).any(|scope| scope.kind.is_exhaustive())
  }
}
