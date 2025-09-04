export function main(unknown) {
  g?.h.i?.j?.(o?.p)?.q.r;

  let a = { b: unknown };
  effect(a?.b?.c, a.c?.d, a.c?.() + 1);
}
