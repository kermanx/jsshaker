export function *f1(a) {
  yield (1+1);
  let unused = yield *a;
  let t = yield 2;
  if (t) {
    effect(1);
  }
}