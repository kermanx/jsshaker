#[macro_export]
macro_rules! init_namespace {
  ($ns:expr, { $($k:expr => $v:expr,)* }) => {
    {
      use $crate::entity::{ObjectProperty, ObjectPropertyValue};
      let mut string_keyed = $ns.string_keyed.borrow_mut();
      $(string_keyed.insert(
        $k,
        ObjectProperty {
          definite: true,
          possible_values: vec![ObjectPropertyValue::Field($v, true)],
          non_existent: Default::default(),
          key: None,
          mangling: None,
        },
      );)*
    }
  };
}

#[macro_export]
macro_rules! init_object {
  ($ns:expr, { $($k:expr => $v:expr,)* }) => {
    {
      use $crate::entity::{ObjectProperty, ObjectPropertyValue};
      let mut string_keyed = $ns.string_keyed.borrow_mut();
      $(string_keyed.insert(
        $k,
        ObjectProperty {
          definite: true,
          possible_values: vec![ObjectPropertyValue::Field($v, false)],
          non_existent: Default::default(),
          key: None,
          mangling: None,
        },
      );)*
    }
  };
}

#[macro_export]
macro_rules! init_map {
  ($map:expr, { $($k:expr => $v:expr,)* }) => {
    {
      $($map.insert($k, $v);)*
    }
  };
}
