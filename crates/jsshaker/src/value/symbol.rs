use oxc::semantic::SymbolId;
use oxc_index::Idx;
use rustc_hash::FxHashMap;
use std::sync::{
  atomic::{AtomicUsize, Ordering},
  LazyLock, Mutex,
};

static SYMBOL_ID: AtomicUsize = AtomicUsize::new(0);

pub fn new_symbol_id() -> SymbolId {
  SymbolId::from_usize(SYMBOL_ID.fetch_add(1, Ordering::Relaxed))
}

static NAMED_SYMBOL_MAP: LazyLock<Mutex<FxHashMap<String, SymbolId>>> =
  LazyLock::new(Default::default);

pub fn new_symbol_id_from_name(name: &str) -> SymbolId {
  let mut map = NAMED_SYMBOL_MAP.lock().unwrap();
  if let Some(symbol_id) = map.get(name) {
    return *symbol_id;
  }
  let symbol_id = SymbolId::from_usize(SYMBOL_ID.fetch_add(1, Ordering::Relaxed));
  map.insert(name.to_string(), symbol_id);
  symbol_id
}
