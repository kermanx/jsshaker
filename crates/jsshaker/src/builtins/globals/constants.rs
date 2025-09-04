use crate::{builtins::Builtins, init_map};

impl Builtins<'_> {
  pub fn init_global_constants(&mut self) {
    let factory = self.factory;

    init_map!(self.globals, {
      "undefined" => factory.undefined,
      "Infinity" => factory.infinity(true),
      "NaN" => factory.nan,
      "eval" => factory.unknown,
      "RegExp" => factory.unknown,
      "Array" => factory.unknown,

      "$$DEBUG$$" => factory.implemented_builtin_fn(
        "debug",
        |analyzer, dep, _this, args| {
          let args = args.destruct_as_array(analyzer, dep, 1, true).0;
          println!("Debug: {:#?}", args[0]);
          analyzer.factory.undefined
        },
      ),
    })
  }
}
