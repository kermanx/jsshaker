import { foo, unused } from 'source' with { type: 'json' };
import 'side-effect-only';
import unused2 from 'default-export-unused';
import de from 'default-export';

export * as t from 'export-all';

export function f0() {
  unused + unused2;
  foo(() => 1);
  return de + "1";
}

export function f1(a) {
  effect(a);
  return 1;
}

export const f2 = (a) => {
  effect(a);
  return 1;
}

export const t = 123;

export default t + "," + f2;

let u = 1;
export { u } from 're-exported-named';
