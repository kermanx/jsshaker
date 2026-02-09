use std::cell::{Cell, RefCell};

use oxc::{allocator::Allocator, semantic::SymbolId};
use rustc_hash::FxHashMap;

use super::LiteralValue;

pub struct SymbolRegistry<'a> {
  counter: Cell<usize>,
  global_symbols: RefCell<FxHashMap<&'a str, &'a LiteralValue<'a>>>,
}

impl<'a> SymbolRegistry<'a> {
  pub fn default() -> Self {
    Self { counter: Cell::new(0), global_symbols: RefCell::new(FxHashMap::default()) }
  }

  pub fn alloc_symbol_id(&self) -> SymbolId {
    let id = self.counter.get();
    self.counter.set(id + 1);
    SymbolId::from_usize(id)
  }

  pub fn get_or_create_global_symbol(
    &self,
    key: &'a str,
    allocator: &'a Allocator,
  ) -> &'a LiteralValue<'a> {
    let mut map = self.global_symbols.borrow_mut();
    if let Some(&symbol) = map.get(key) {
      return symbol;
    }

    let symbol_id = self.alloc_symbol_id();
    let symbol = allocator.alloc(LiteralValue::Symbol(symbol_id));
    map.insert(key, symbol);
    symbol
  }
}
