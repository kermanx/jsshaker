use super::CustomDepTrait;
use crate::analyzer::Analyzer;
use std::{cell::Cell, fmt::Debug, marker::PhantomData};

pub struct OnceDep<'a, T: CustomDepTrait<'a> + 'a>(Cell<Option<T>>, PhantomData<&'a ()>);

impl<'a, T: CustomDepTrait<'a> + 'a> Debug for OnceDep<'a, T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("OnceDep").finish()
  }
}

impl<'a, T: CustomDepTrait<'a> + 'a> CustomDepTrait<'a> for OnceDep<'a, T> {
  fn consume(&self, analyzer: &mut Analyzer<'a>) {
    if let Some(value) = self.0.take() {
      value.consume(analyzer)
    }
  }
}

impl<'a, T: CustomDepTrait<'a> + 'a> OnceDep<'a, T> {
  pub fn new(value: T) -> Self {
    Self(Cell::new(Some(value)), PhantomData)
  }
}
