#[macro_export]
macro_rules! init_namespace {
  ($ns:expr, $factory:expr, { $($k:expr => $v:expr,)* }) => {
    {
      use $crate::value::{ObjectProperty, ObjectPropertyValue, PropertyKeyValue};
      use $crate::dep::DepCollector;
      let mut keyed = $ns.keyed.borrow_mut();
      $(keyed.insert(
        PropertyKeyValue::String($k),
        ObjectProperty {
          definite: true,
          enumerable: false,
          possible_values:  $factory.vec1(ObjectPropertyValue::Field($v, true)),
          non_existent: DepCollector::new($factory.vec()),
          key: None,
          mangling: None,
        },
      );)*
    }
  };
}

#[macro_export]
macro_rules! init_object {
  ($obj:expr, $factory:expr, { $($k:expr => $v:expr,)* }) => {
    {
      use $crate::value::{ObjectProperty, ObjectPropertyValue, PropertyKeyValue};
      use $crate::dep::DepCollector;
      let mut keyed = $obj.keyed.borrow_mut();
      $(keyed.insert(
        PropertyKeyValue::String($k),
        ObjectProperty {
          definite: true,
          enumerable: true,
          possible_values: $factory.vec1(ObjectPropertyValue::Field($v, false)),
          non_existent: DepCollector::new($factory.vec()),
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
