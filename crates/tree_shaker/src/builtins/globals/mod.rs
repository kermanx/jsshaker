mod constants;
mod object_constructor;

use super::Builtins;

impl Builtins<'_> {
  pub fn init_globals(&mut self) {
    self.init_global_constants();
    self.init_object_constructor();
  }
}
