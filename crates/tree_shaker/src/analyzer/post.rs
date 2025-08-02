use crate::module::ModuleId;

use super::Analyzer;

impl Analyzer<'_> {
  pub fn post_analysis(&mut self) {
    self.module_stack.push(ModuleId::new(0));

    self.consume_exports(ModuleId::new(0));

    let mut round = 0usize;
    loop {
      round += 1;
      if round > 1000 {
        panic!("Possible infinite loop in post analysis");
      }

      let mut dirty = false;
      dirty |= self.call_exhaustive_callbacks();
      dirty |= self.post_analyze_handle_conditional();
      // dirty |= self.post_analyze_handle_loops();
      dirty |= self.post_analyze_handle_folding();
      if !dirty {
        break;
      }
    }

    self.module_stack.pop();

    #[cfg(feature = "flame")]
    {
      self.scoping.call.pop().unwrap().scope_guard.end();
      flamescope::dump(&mut std::fs::File::create("flamescope.json").unwrap()).unwrap();
    }
  }
}
