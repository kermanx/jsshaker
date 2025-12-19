use std::mem;

use rustc_hash::FxHashMap;

use crate::{
  Analyzer,
  dep::{Dep, DepAtom},
};

pub type AssocDepMap<'a> = FxHashMap<DepAtom, Vec<Dep<'a>>>;

impl<'a> Analyzer<'a> {
  pub fn add_assoc_dep(&mut self, base: impl Into<DepAtom>, dep: Dep<'a>) {
    self.assoc_deps.entry(base.into()).or_default().push(dep);
  }

  pub fn post_analyze_handle_assoc_deps(&mut self) -> bool {
    let mut to_consume = vec![];
    self.assoc_deps.retain(|base, deps: &mut Vec<Dep<'a>>| {
      if self.referred_deps.is_referred(*base) {
        to_consume.push(mem::take(deps));
        false
      } else {
        true
      }
    });
    if to_consume.is_empty() {
      return false;
    }
    self.consume(to_consume);
    true
  }
}
