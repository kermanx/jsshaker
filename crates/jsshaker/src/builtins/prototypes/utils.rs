#[macro_export]
macro_rules! insert_prototype_property {
  ($p:expr, $k:literal : $v:expr) => {
    $p.insert_string_keyed($k, false, $v)
  };
  ($p:expr, $k:literal => $v:expr) => {
    $p.insert_string_keyed($k, true, $v)
  };
}

#[macro_export]
macro_rules! init_prototype {
  ($name:expr, $p:expr, { $($k:literal $s:tt $v:expr),* $(,)? }) => {
    {
      let mut prototype = $p.with_name($name);
      $($crate::insert_prototype_property!(prototype, $k $s $v);)*
      prototype
    }
  };
}
