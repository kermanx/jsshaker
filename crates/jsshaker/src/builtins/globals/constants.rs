use crate::{
  builtins::Builtins,
  init_map,
  value::{ObjectPropertyValue, ObjectPrototype},
};

impl Builtins<'_> {
  pub fn init_global_constants(&mut self) {
    let factory = self.factory;

    let unknown_object = {
      let unknown_object = factory.builtin_object(ObjectPrototype::Unknown(factory.no_dep), false);
      unknown_object
        .unknown
        .borrow_mut()
        .possible_values
        .push(ObjectPropertyValue::Field(factory.unknown, true));
      unknown_object.into()
    };

    init_map!(self.globals, {
      // Value properties
      "undefined" => factory.undefined,
      "Infinity" => factory.number(f64::INFINITY),
      "NaN" => factory.nan,
      "globalThis" => unknown_object,

      // Function properties
      "eval" => unknown_object,
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
      "Function" => unknown_object,
      "Boolean" => unknown_object,

      // Error objects
      "Error" => unknown_object,
      "AggregateError" => unknown_object,
      "EvalError" => unknown_object,
      "RangeError" => unknown_object,
      "ReferenceError" => unknown_object,
      "SyntaxError" => unknown_object,
      "TypeError" => unknown_object,
      "URIError" => unknown_object,

      // Numbers and dates
      "Number" => unknown_object,
      "BigInt" => unknown_object,

      // Text processing
      "String" => unknown_object,
      "RegExp" => unknown_object,

      // Indexed collections (Array is in array_constructor.rs)
      "Int8Array" => unknown_object,
      "Uint8Array" => unknown_object,
      "Uint8ClampedArray" => unknown_object,
      "Int16Array" => unknown_object,
      "Uint16Array" => unknown_object,
      "Int32Array" => unknown_object,
      "Uint32Array" => unknown_object,
      "BigInt64Array" => unknown_object,
      "BigUint64Array" => unknown_object,
      "Float32Array" => unknown_object,
      "Float64Array" => unknown_object,

      // Keyed collections
      "Map" => unknown_object,
      "Set" => unknown_object,
      "WeakMap" => unknown_object,
      "WeakSet" => unknown_object,

      // Structured data
      "ArrayBuffer" => unknown_object,
      "SharedArrayBuffer" => unknown_object,
      "DataView" => unknown_object,
      "Atomics" => unknown_object,
      // JSON is in json_object.rs

      // Managing memory
      "WeakRef" => unknown_object,
      "FinalizationRegistry" => unknown_object,

      // Control abstraction objects
      "Iterator" => unknown_object,
      "AsyncIterator" => unknown_object,
      "Promise" => unknown_object,
      "GeneratorFunction" => unknown_object,
      "AsyncGeneratorFunction" => unknown_object,
      "Generator" => unknown_object,
      "AsyncGenerator" => unknown_object,
      "AsyncFunction" => unknown_object,

      // Reflection
      "Reflect" => unknown_object,
      "Proxy" => unknown_object,

      // Internationalization
      "Intl" => unknown_object,
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
