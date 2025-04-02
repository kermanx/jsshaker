use std::mem;

use rustc_hash::FxHashMap;

use crate::{analyzer::Analyzer, dep::DepAtom, transformer::Transformer};

pub struct DataPlaceholder<'a> {
  _phantom: std::marker::PhantomData<&'a ()>,
}

pub type ExtraData<'a> = FxHashMap<DepAtom, Box<DataPlaceholder<'a>>>;

#[derive(Debug, Default)]
pub struct StatementVecData {
  pub last_stmt: Option<usize>,
}

impl<'a> Analyzer<'a> {
  pub fn get_data_or_insert_with<D: 'a>(
    &mut self,
    key: impl Into<DepAtom>,
    default: impl FnOnce() -> D,
  ) -> &'a mut D {
    let boxed =
      self.data.entry(key.into()).or_insert_with(|| unsafe { mem::transmute(Box::new(default())) });
    unsafe { mem::transmute(boxed.as_mut()) }
  }

  pub fn load_data<D: Default + 'a>(&mut self, key: impl Into<DepAtom>) -> &'a mut D {
    self.get_data_or_insert_with(key, Default::default)
  }
}

impl<'a> Transformer<'a> {
  pub fn get_data<D: Default + 'a>(&self, key: impl Into<DepAtom>) -> &'a D {
    const { assert!(!std::mem::needs_drop::<D>(), "Cannot allocate Drop type in arena") };

    let existing = self.data.get(&key.into());
    match existing {
      Some(boxed) => unsafe { mem::transmute::<&DataPlaceholder<'_>, &D>(boxed.as_ref()) },
      None => self.allocator.alloc(D::default()),
    }
  }
}
