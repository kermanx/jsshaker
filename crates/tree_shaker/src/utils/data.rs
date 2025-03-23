use rustc_hash::FxHashMap;
use std::collections::BTreeSet;

use super::dep_id::DepId;

pub struct DataPlaceholder<'a> {
  _phantom: std::marker::PhantomData<&'a ()>,
}

pub type ExtraData<'a> = FxHashMap<DepId, Box<DataPlaceholder<'a>>>;

pub type Diagnostics = BTreeSet<String>;

#[derive(Debug, Default)]
pub struct StatementVecData {
  pub last_stmt: Option<usize>,
}
