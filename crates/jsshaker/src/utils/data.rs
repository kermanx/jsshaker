use oxc::allocator::Allocator;
use rustc_hash::FxHashMap;

use crate::{analyzer::Analyzer, dep::DepAtom, transformer::Transformer};

pub type ExtraData<'a> = FxHashMap<DepAtom, *mut ()>;

#[derive(Debug, Default)]
pub struct StatementVecData {
  pub last_stmt: Option<usize>,
}

pub trait DefaultIn<'a> {
  fn default_in(allocator: &'a Allocator) -> Self;
}

impl<'a, T: Default> DefaultIn<'a> for T {
  fn default_in(_allocator: &'a Allocator) -> Self {
    T::default()
  }
}

impl<'a> Analyzer<'a> {
  pub fn load_data<D: DefaultIn<'a> + 'a>(&mut self, key: impl Into<DepAtom>) -> &'a mut D {
    let data = self
      .data
      .entry(key.into())
      .or_insert_with(|| self.allocator.alloc(D::default_in(self.allocator)) as *mut D as *mut ());
    unsafe { &mut *(*data as *mut D) }
  }
}

impl<'a> Transformer<'a> {
  pub fn get_data<D: DefaultIn<'a> + 'a>(&self, key: impl Into<DepAtom>) -> &'a D {
    const { assert!(!std::mem::needs_drop::<D>(), "Cannot allocate Drop type in arena") };

    let existing = self.data.get(&key.into());
    match existing {
      Some(data) => unsafe { &*(*data as *const D) },
      None => self.allocator.alloc(D::default_in(self.allocator)),
    }
  }
}
