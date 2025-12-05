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
        |analyzer, _dep, _this, args| {
          println!("Debug: {:#?}", args.get(analyzer, 0));
          analyzer.factory.undefined
        },
      ),
    })
  }
}
