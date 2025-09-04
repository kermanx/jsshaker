export function f1() {
  let a = 1;
  a++;
  effect(a, a++, ++a);
  a++;
  effect(a);
  if (unknown) a++;
  effect(a);
}

export function f2() {
  let b = { value: 1 };
  b.value++;
  effect(b.value, b.value++, ++b.value);
  b.value++;
  effect(b.value);
  if (unknown) b.value++;
  effect(b.value);
}

export function f3() {
  let c = { value: 1 };
  c.value++;
  c[unknown]++;
  c[(effect(), 'value')]++;
}
