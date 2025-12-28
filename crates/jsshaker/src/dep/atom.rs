use std::{
  fmt::Debug,
  hash::Hash,
  sync::atomic::{AtomicUsize, Ordering},
};

use oxc::span::{GetSpan, Span};
use rustc_hash::FxHashSet;

use crate::{analyzer::Analyzer, ast::AstKind2, dep::CustomDepTrait, transformer::Transformer};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct DepAtom((u8, usize));

impl Debug for DepAtom {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    AstKind2::from(*self).fmt(f)
  }
}

impl<'a> CustomDepTrait<'a> for DepAtom {
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    analyzer.refer_dep(*self);
  }
}

impl<'a> From<AstKind2<'a>> for DepAtom {
  fn from(node: AstKind2<'a>) -> Self {
    DepAtom(unsafe { std::mem::transmute(node) })
  }
}

impl From<DepAtom> for AstKind2<'_> {
  fn from(val: DepAtom) -> Self {
    unsafe { std::mem::transmute(val.0) }
  }
}

impl GetSpan for DepAtom {
  fn span(&self) -> Span {
    let ast_kind: AstKind2<'_> = (*self).into();
    ast_kind.span()
  }
}

impl PartialEq for AstKind2<'_> {
  fn eq(&self, other: &Self) -> bool {
    DepAtom::from(*self) == DepAtom::from(*other)
  }
}
impl Eq for AstKind2<'_> {}
impl Hash for AstKind2<'_> {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    DepAtom::from(*self).hash(state);
  }
}

static COUNTER: AtomicUsize = AtomicUsize::new(0);

impl DepAtom {
  pub fn from_counter() -> Self {
    AstKind2::Index(COUNTER.fetch_add(1, Ordering::Relaxed)).into()
  }
}

pub struct ReferredDeps(FxHashSet<DepAtom>);

impl Default for ReferredDeps {
  fn default() -> Self {
    Self(FxHashSet::from_iter([AstKind2::Environment.into()]))
  }
}

impl ReferredDeps {
  pub fn refer_dep(&mut self, dep: impl Into<DepAtom>) {
    self.0.insert(dep.into());
  }

  pub fn is_referred(&self, dep: impl Into<DepAtom>) -> bool {
    self.0.contains(&dep.into())
  }
}

impl Analyzer<'_> {
  pub fn refer_dep(&mut self, dep: impl Into<DepAtom>) {
    self.referred_deps.refer_dep(dep);
  }

  pub fn is_referred(&self, dep: impl Into<DepAtom>) -> bool {
    self.referred_deps.is_referred(dep)
  }
}

impl Transformer<'_> {
  pub fn is_referred(&self, dep: impl Into<DepAtom>) -> bool {
    self.referred_deps.is_referred(dep)
  }
}
