mod collector;
mod impls;
mod lazy;
mod once;

use crate::{analyzer::Analyzer, entity::EntityFactory};
pub use collector::*;
pub use lazy::*;
use once::OnceConsumable;
use oxc::allocator;
use std::fmt::Debug;

pub trait ConsumableTrait<'a>: Debug {
  fn consume(&self, analyzer: &mut Analyzer<'a>);
}

#[derive(Debug, Clone, Copy)]
pub struct Consumable<'a>(pub &'a (dyn ConsumableTrait<'a> + 'a));

pub type ConsumableVec<'a> = allocator::Vec<'a, Consumable<'a>>;

impl<'a> EntityFactory<'a> {
  pub fn consumable_no_once(&self, dep: impl ConsumableTrait<'a> + 'a) -> Consumable<'a> {
    Consumable(self.alloc(dep))
  }

  pub fn consumable_once(&self, dep: impl ConsumableTrait<'a> + 'a) -> Consumable<'a> {
    self.consumable_no_once(OnceConsumable::new(dep))
  }

  pub fn consumable(&self, dep: impl ConsumableTrait<'a> + 'a) -> Consumable<'a> {
    self.consumable_once(dep)
  }
}

impl<'a> Analyzer<'a> {
  pub fn consume(&mut self, dep: impl ConsumableTrait<'a> + 'a) {
    dep.consume(self);
  }

  pub fn consumable(&self, dep: impl ConsumableTrait<'a> + 'a) -> Consumable<'a> {
    self.factory.consumable(dep)
  }
}
