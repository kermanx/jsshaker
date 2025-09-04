#[macro_export]
macro_rules! use_consumed_flag {
  ($self: expr) => {
    if $self.consumed.replace(true) {
      return;
    }
  };
}
