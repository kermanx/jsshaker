mod array_constructor;
mod constants;
mod json_object;
mod math_object;
mod object_constructor;
mod symbol_constructor;

use super::Builtins;

impl Builtins<'_> {
  pub fn init_globals(&mut self) {
    self.init_global_constants();
    self.init_object_constructor();
    self.init_symbol_constructor();
    self.init_array_constructor();
    self.init_json_object();
    self.init_math_object();
  }
}
