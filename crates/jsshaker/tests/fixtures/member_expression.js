export function main(unknown) {
  let a = { b: 1 };
  effect(a.b);

  effect(unknown.a);

  let c = { d: unknown };
  c[(effect(), "d")]
  effect(c[(effect(), "d")]);

  global[effect()];
  effect(global[effect()]);
}
