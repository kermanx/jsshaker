use crate::{
  builtins::Builtins,
  init_map,
  value::{ObjectPropertyValue, ObjectPrototype},
};

impl Builtins<'_> {
  pub fn init_global_constants(&mut self) {
    let factory = self.factory;

    let make_builtin_object = || {
      let unknown_object = factory.builtin_object(ObjectPrototype::Unknown(factory.no_dep));
      unknown_object
        .unknown
        .borrow_mut()
        .possible_values
        .push(ObjectPropertyValue::Field(factory.unknown, true));
      unknown_object
    };
    let builtin_object = make_builtin_object().into();
    let builtin_function = {
      let object = make_builtin_object();
      object.is_builtin_function = true;
      object.into()
    };

    init_map!(self.globals, {
      // Value properties
      "undefined" => factory.undefined,
      "Infinity" => factory.number(f64::INFINITY),
      "NaN" => factory.nan,
      "globalThis" => factory.unknown_truthy,

      // Function properties
      "eval" => builtin_function,
      "isFinite" => factory.pure_fn_returns_boolean,
      "isNaN" => factory.pure_fn_returns_boolean,
      "parseFloat" => factory.pure_fn_returns_number,
      "parseInt" => factory.pure_fn_returns_number,
      "decodeURI" => factory.pure_fn_returns_string,
      "decodeURIComponent" => factory.pure_fn_returns_string,
      "encodeURI" => factory.pure_fn_returns_string,
      "encodeURIComponent" => factory.pure_fn_returns_string,
      "setTimeout" => builtin_function,
      "clearTimeout" => builtin_function,
      "setInterval" => builtin_function,
      "clearInterval" => builtin_function,
      "setImmediate" => builtin_function,
      "clearImmediate" => builtin_function,
      "queueMicrotask" => builtin_function,
      "requestAnimationFrame" => builtin_function,
      "structuredClone" => builtin_function,
      // Deprecated but still part of the standard
      "include" => factory.pure_fn_returns_string,
      "uninclude" => factory.pure_fn_returns_string,

      // Fundamental objects
      "Function" => builtin_function,
      "Boolean" => builtin_function,

      // Error objects
      "Error" => builtin_function,
      "AggregateError" => builtin_function,
      "EvalError" => builtin_function,
      "RangeError" => builtin_function,
      "ReferenceError" => builtin_function,
      "SyntaxError" => builtin_function,
      "TypeError" => builtin_function,
      "URIError" => builtin_function,

      // Numbers and dates
      "Number" => builtin_function,
      "BigInt" => builtin_function,

      // Text processing
      "String" => builtin_function,
      "RegExp" => builtin_function,

      // Indexed collections (Array is in array_constructor.rs)
      "Int8Array" => builtin_function,
      "Uint8Array" => builtin_function,
      "Uint8ClampedArray" => builtin_function,
      "Int16Array" => builtin_function,
      "Uint16Array" => builtin_function,
      "Int32Array" => builtin_function,
      "Uint32Array" => builtin_function,
      "BigInt64Array" => builtin_function,
      "BigUint64Array" => builtin_function,
      "Float32Array" => builtin_function,
      "Float64Array" => builtin_function,

      // Keyed collections
      "Map" => builtin_function,
      "Set" => builtin_function,
      "WeakMap" => builtin_function,
      "WeakSet" => builtin_function,

      // Structured data
      "ArrayBuffer" => builtin_function,
      "SharedArrayBuffer" => builtin_function,
      "DataView" => builtin_function,
      "Atomics" => builtin_object,
      // JSON is in json_object.rs

      // Managing memory
      "WeakRef" => builtin_function,
      "FinalizationRegistry" => builtin_function,

      // Control abstraction objects
      "Iterator" => builtin_function,
      "AsyncIterator" => builtin_function,
      "Promise" => builtin_function,
      "GeneratorFunction" => builtin_function,
      "AsyncGeneratorFunction" => builtin_function,
      "Generator" => builtin_function,
      "AsyncGenerator" => builtin_function,
      "AsyncFunction" => builtin_function,

      // Reflection
      "Reflect" => builtin_object,
      "Proxy" => builtin_function,

      // Internationalization
      "Intl" => builtin_object,
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
