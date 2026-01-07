use std::{
  hash::{Hash, Hasher},
  mem,
  rc::Rc,
};

use oxc::allocator;

use crate::{
  analyzer::{Analyzer, rw_tracking::ReadWriteTarget},
  entity::Entity,
  scope::{CfScopeKind, cf_scope::DeoptimizeState},
  utils::ast::AstKind2,
};

#[derive(Debug)]
pub struct ExhaustiveData<'a> {
  pub clean: bool,
  pub temp_deps: Option<allocator::HashSet<'a, ReadWriteTarget<'a>>>,
  pub register_deps: Option<allocator::HashSet<'a, ReadWriteTarget<'a>>>,
}

#[derive(Clone)]
pub struct ExhaustiveCallback<'a> {
  pub handler: Rc<dyn Fn(&mut Analyzer<'a>) -> Entity<'a> + 'a>,
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
    let runner = Rc::new(move |analyzer: &mut Analyzer<'a>| {
      runner(analyzer);
      analyzer.factory.never
    });

    self.exec_exhaustively("loop", true, false, runner.clone());

    let cf_scope = self.cf_scope();
    if cf_scope.deoptimize_state != DeoptimizeState::DeoptimizedClean
      && cf_scope.deps.may_not_deoptimized()
    {
      runner(self);
    }
  }

  pub fn exec_consumed_fn(
    &mut self,
    kind: &str,
    runner: impl Fn(&mut Analyzer<'a>) -> Entity<'a> + 'a,
  ) -> Entity<'a> {
    let runner = Rc::new(move |analyzer: &mut Analyzer<'a>| {
      analyzer.scoping.current_callsite = AstKind2::ENVIRONMENT;
      let ret_val = runner(analyzer);
      analyzer.consume(ret_val);
      ret_val
    });
    self.exec_exhaustively(kind, true, true, runner)
  }

  pub fn exec_async_or_generator_fn(
    &mut self,
    runner: impl Fn(&mut Analyzer<'a>) -> Entity<'a> + 'a,
  ) -> Entity<'a> {
    self.exec_exhaustively("async/generator", false, true, Rc::new(runner))
  }

  fn exec_exhaustively(
    &mut self,
    _kind: &str,
    drain: bool,
    register: bool,
    runner: Rc<dyn Fn(&mut Analyzer<'a>) -> Entity<'a> + 'a>,
  ) -> Entity<'a> {
    self.push_cf_scope(
      CfScopeKind::Exhaustive(self.allocator.alloc(ExhaustiveData {
        clean: true,
        temp_deps: drain.then(|| allocator::HashSet::new_in(self.allocator)),
        register_deps: register.then(|| allocator::HashSet::new_in(self.allocator)),
      })),
      true,
    );
    let mut round_counter = 0;
    let mut first_ret = None;
    loop {
      // println!("_kind: {}", _kind);

      self.cf_scope_mut().exited = None;
      #[cfg(feature = "flame")]
      let _scope_guard = flame::start_guard(format!(
        "!{_kind}@{:06X} x{}",
        (Rc::as_ptr(&runner) as *const () as usize) & 0xFFFFFF,
        round_counter
      ));

      let ret = runner(self);
      first_ret.get_or_insert(ret);

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
    first_ret.unwrap()
  }

  fn register_exhaustive_callbacks(
    &mut self,
    drain: bool,
    handler: Rc<dyn Fn(&mut Analyzer<'a>) -> Entity<'a> + 'a>,
    deps: allocator::HashSet<ReadWriteTarget<'a>>,
  ) {
    for id in deps {
      self
        .exhaustive_callbacks
        .entry(id)
        .or_default()
        .insert(ExhaustiveCallback { handler: handler.clone(), drain });
    }
  }

  pub fn request_exhaustive_callbacks(&mut self, id: ReadWriteTarget<'a>) -> bool {
    let mut found = false;
    let mut do_request = |id: ReadWriteTarget<'a>| {
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
        // let old_count = self.deoptimized_atoms.debug_count();
        let ExhaustiveCallback { handler: runner, drain } = runner;
        self.exec_exhaustively("dep", drain, true, runner.clone());
        // let new_count = self.deoptimized_atoms.debug_count();
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
