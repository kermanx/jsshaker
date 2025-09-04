export function f1() {
  let a = 1;
  a = 2;
  return a;
}

export function f2(a) {
  a = 2;
  return a;
}

export function f3(a, b) {
  if (b) a = 2;
  return a;
}

export function f4(a, b) {
  if (b) a = 2;
  a = 3
  return a;
}

export function f5(a, b) {
  a = b;
  return a;
}

export function f6(a, b) {
  a.p = b;
  global.p = 1;
  (1).p = 2;
  (1).p = effect;
  return a;
}

export function f7(a) {
  let x, y, z, w;
  ({ x, a: y = 1, b: { c: z = 2 }, ...w } = a);
  
  let obj = { };
  ({ a: obj.a, b: obj.b } = { a: 'a', b: a });
  effect(obj.a);
}
