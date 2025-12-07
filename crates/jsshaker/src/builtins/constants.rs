use crate::value::ObjectId;

// Builtin object ids
pub const IMPORT_META_OBJECT_ID: ObjectId = ObjectId::from_raw(1u32);
pub const REACT_NAMESPACE_OBJECT_ID: ObjectId = ObjectId::from_raw(2u32);
pub const REACT_JSX_RUNTIME_NAMESPACE_OBJECT_ID: ObjectId = ObjectId::from_raw(3u32);
pub const OBJECT_CONSTRUCTOR_OBJECT_ID: ObjectId = ObjectId::from_raw(4u32);
pub const SYMBOL_CONSTRUCTOR_OBJECT_ID: ObjectId = ObjectId::from_raw(5u32);
