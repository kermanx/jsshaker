use crate::{
  builtins::Builtins,
  init_map,
  value::{ObjectPropertyValue, ObjectPrototype},
};

impl Builtins<'_> {
  pub fn init_global_constants(&mut self) {
    let factory = self.factory;

    let make_unknown_object = || {
      let unknown_object = factory.builtin_object(ObjectPrototype::Unknown(factory.no_dep), false);
      unknown_object
        .unknown
        .borrow_mut()
        .possible_values
        .push(ObjectPropertyValue::Field(factory.unknown, true));
      unknown_object
    };
    let unknown_object = make_unknown_object().into();
    let unknown_function = {
      let object = make_unknown_object();
      object.is_builtin_function = true;
      object.into()
    };

    init_map!(self.globals, {
      // Value properties
      "undefined" => factory.undefined,
      "Infinity" => factory.number(f64::INFINITY),
      "NaN" => factory.nan,
      "globalThis" => unknown_object,

      // Function properties
      "eval" => unknown_function,
      "isFinite" => factory.pure_fn_returns_boolean,
      "isNaN" => factory.pure_fn_returns_boolean,
      "parseFloat" => factory.pure_fn_returns_number,
      "parseInt" => factory.pure_fn_returns_number,
      "decodeURI" => factory.pure_fn_returns_string,
      "decodeURIComponent" => factory.pure_fn_returns_string,
      "encodeURI" => factory.pure_fn_returns_string,
      "encodeURIComponent" => factory.pure_fn_returns_string,
      "setTimeout" => unknown_function,
      "clearTimeout" => unknown_function,
      "setInterval" => unknown_function,
      "clearInterval" => unknown_function,
      "setImmediate" => unknown_function,
      "clearImmediate" => unknown_function,
      "queueMicrotask" => unknown_function,
      "requestAnimationFrame" => unknown_function,
      "structuredClone" => unknown_function,
      // Deprecated but still part of the standard
      "escape" => factory.pure_fn_returns_string,
      "unescape" => factory.pure_fn_returns_string,

      // Fundamental objects
      "Function" => unknown_function,
      "Boolean" => unknown_function,

      // Error objects
      "Error" => unknown_function,
      "AggregateError" => unknown_function,
      "EvalError" => unknown_function,
      "RangeError" => unknown_function,
      "ReferenceError" => unknown_function,
      "SyntaxError" => unknown_function,
      "TypeError" => unknown_function,
      "URIError" => unknown_function,

      // Numbers and dates
      "Number" => unknown_function,
      "BigInt" => unknown_function,

      // Text processing
      "String" => unknown_function,
      "RegExp" => unknown_function,

      // Indexed collections (Array is in array_constructor.rs)
      "Int8Array" => unknown_function,
      "Uint8Array" => unknown_function,
      "Uint8ClampedArray" => unknown_function,
      "Int16Array" => unknown_function,
      "Uint16Array" => unknown_function,
      "Int32Array" => unknown_function,
      "Uint32Array" => unknown_function,
      "BigInt64Array" => unknown_function,
      "BigUint64Array" => unknown_function,
      "Float32Array" => unknown_function,
      "Float64Array" => unknown_function,

      // Keyed collections
      "Map" => unknown_function,
      "Set" => unknown_function,
      "WeakMap" => unknown_function,
      "WeakSet" => unknown_function,

      // Structured data
      "ArrayBuffer" => unknown_function,
      "SharedArrayBuffer" => unknown_function,
      "DataView" => unknown_function,
      "Atomics" => unknown_object,
      // JSON is in json_object.rs

      // Managing memory
      "WeakRef" => unknown_function,
      "FinalizationRegistry" => unknown_function,

      // Control abstraction objects
      "Iterator" => unknown_function,
      "AsyncIterator" => unknown_function,
      "Promise" => unknown_function,
      "GeneratorFunction" => unknown_function,
      "AsyncGeneratorFunction" => unknown_function,
      "Generator" => unknown_function,
      "AsyncGenerator" => unknown_function,
      "AsyncFunction" => unknown_function,

      // Reflection
      "Reflect" => unknown_object,
      "Proxy" => unknown_function,

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
