use oxc::semantic::SymbolId;
use std::mem;

// Builtin object ids
pub const IMPORT_META_OBJECT_ID: SymbolId = unsafe { mem::transmute(1u32) };
pub const REACT_NAMESPACE_OBJECT_ID: SymbolId = unsafe { mem::transmute(2u32) };
pub const REACT_JSX_RUNTIME_NAMESPACE_OBJECT_ID: SymbolId = unsafe { mem::transmute(3u32) };
pub const OBJECT_CONSTRUCTOR_OBJECT_ID: SymbolId = unsafe { mem::transmute(4u32) };
pub const SYMBOL_CONSTRUCTOR_OBJECT_ID: SymbolId = unsafe { mem::transmute(5u32) };
