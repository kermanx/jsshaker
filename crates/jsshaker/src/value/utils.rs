#[macro_export]
macro_rules! use_included_flag {
  ($self: expr) => {
    if $self.included.replace(true) {
      return;
    }
  };
}
