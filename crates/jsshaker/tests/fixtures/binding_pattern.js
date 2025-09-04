export function simple(unknown) {
  let { a } = { a: 1 };
  effect(a);

  let { b: c } = { b: 2 };
  effect(c);

  let { d = 3 } = { d: undefined };
  effect(d);

  let { [0 + 1] : e } = { 1: 4 };
  effect(e);

  // Destructing unknown has effect
  let { g: { h, i: { j } } } = unknown;
}

export function with_rest(unknown) {
  let { a, ...rest } = { a: 1, b: unknown, c: 2 };
  effect(rest.a, rest.b, rest.c);

  let { b, ...rest2 } = unknown;
  let { c, ...rest3 } = rest2;

  let { d, e, ...unused } = { d: unknown, e: unknown };
  effect(d);
}
