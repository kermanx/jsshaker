export const l1 = await 1;
export const l2 = await 1 + "a";

export const l3 = await unknown;
export const l4 = await (unknown + "a");

async function pure() {
  return 1;
}

export const r1 = await pure();
export const r2 = pure();

async function nested_pure() {
  return pure();
}

export const r3 = await nested_pure();

async function effect() {
  await something;
  return 1;
}

export const r4 = await effect();

function effect2() {
  return something;
}

export const r5 = await effect2();

export async function pure_but_complex(unknown) {
  const f = async () => unknown;
  return await f();
}
