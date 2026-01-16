use crate::{builtins::Builtins, init_map};

impl Builtins<'_> {
  pub fn init_global_constants(&mut self) {
    let factory = self.factory;

    init_map!(self.globals, {
      // Value properties
      "undefined" => factory.undefined,
      "Infinity" => factory.number(f64::INFINITY),
      "NaN" => factory.nan,
      "globalThis" => factory.unknown,

      // Function properties
      "eval" => factory.unknown,
      "isFinite" => factory.pure_fn_returns_boolean,
      "isNaN" => factory.pure_fn_returns_boolean,
      "parseFloat" => factory.pure_fn_returns_number,
      "parseInt" => factory.pure_fn_returns_number,
      "decodeURI" => factory.pure_fn_returns_string,
      "decodeURIComponent" => factory.pure_fn_returns_string,
      "encodeURI" => factory.pure_fn_returns_string,
      "encodeURIComponent" => factory.pure_fn_returns_string,
      // Deprecated but still part of the standard
      "escape" => factory.pure_fn_returns_string,
      "unescape" => factory.pure_fn_returns_string,

      // Fundamental objects
      "Function" => factory.unknown,
      "Boolean" => factory.unknown,

      // Error objects
      "Error" => factory.unknown,
      "AggregateError" => factory.unknown,
      "EvalError" => factory.unknown,
      "RangeError" => factory.unknown,
      "ReferenceError" => factory.unknown,
      "SyntaxError" => factory.unknown,
      "TypeError" => factory.unknown,
      "URIError" => factory.unknown,

      // Numbers and dates
      "Number" => factory.unknown,
      "BigInt" => factory.unknown,

      // Text processing
      "String" => factory.unknown,
      "RegExp" => factory.unknown,

      // Indexed collections (Array is in array_constructor.rs)
      "Int8Array" => factory.unknown,
      "Uint8Array" => factory.unknown,
      "Uint8ClampedArray" => factory.unknown,
      "Int16Array" => factory.unknown,
      "Uint16Array" => factory.unknown,
      "Int32Array" => factory.unknown,
      "Uint32Array" => factory.unknown,
      "BigInt64Array" => factory.unknown,
      "BigUint64Array" => factory.unknown,
      "Float32Array" => factory.unknown,
      "Float64Array" => factory.unknown,

      // Keyed collections
      "Map" => factory.unknown,
      "Set" => factory.unknown,
      "WeakMap" => factory.unknown,
      "WeakSet" => factory.unknown,

      // Structured data
      "ArrayBuffer" => factory.unknown,
      "SharedArrayBuffer" => factory.unknown,
      "DataView" => factory.unknown,
      "Atomics" => factory.unknown,
      // JSON is in json_object.rs

      // Managing memory
      "WeakRef" => factory.unknown,
      "FinalizationRegistry" => factory.unknown,

      // Control abstraction objects
      "Iterator" => factory.unknown,
      "AsyncIterator" => factory.unknown,
      "Promise" => factory.unknown,
      "GeneratorFunction" => factory.unknown,
      "AsyncGeneratorFunction" => factory.unknown,
      "Generator" => factory.unknown,
      "AsyncGenerator" => factory.unknown,
      "AsyncFunction" => factory.unknown,

      // Reflection
      "Reflect" => factory.unknown,
      "Proxy" => factory.unknown,

      // Internationalization
      "Intl" => factory.unknown,
    });

    // Debug helpers (non-standard)
    #[cfg(debug_assertions)]
    init_map!(self.globals, {
      "$$DEBUG$$" => factory.implemented_builtin_fn(
        "$$DEBUG$$",
        |analyzer, _dep, _this, args| {
          println!("Debug: {:#?}", args.get(analyzer, 0));
          analyzer.factory.undefined
        },
      ),
      "$$TRACE$$" => factory.implemented_builtin_fn(
        "$$TRACE$$",
        |analyzer, _dep, _this, args| {
          println!("Trace: {:#?}", args.get(analyzer, 0).get_literal(analyzer).unwrap().to_string(analyzer.allocator));
          analyzer.factory.undefined
        },
      ),
    })
  }
}
