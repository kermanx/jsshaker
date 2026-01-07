#[macro_export]
macro_rules! insert_prototype_property {
  ($p:expr, $k:literal, $v:expr) => {
    $p.insert_string_keyed($k, $v)
  };
}

#[macro_export]
macro_rules! init_prototype {
  ($name:expr, $p:expr, { $($k:expr => $v:expr,)* }) => {
    {
      let mut prototype = $p.with_name($name);
      $(prototype.insert_string_keyed($k, $v);)*
      prototype
    }
  };
}
