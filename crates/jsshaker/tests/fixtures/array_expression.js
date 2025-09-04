export function main(unknown) {
  const arr1 = [1+1, "a"+"b"];
  effect(arr1[0], arr1[1]);

  const arr2 = [2+2, ...[3+3], , ...unknown, unknown, ...[...[1,2+2]]];
  effect(arr2);

  const unused1 = [1+1, ...[], , effect()];
  const unused2 = [1+1, ...[effect()]];
  const unused3 = [1+1, ...[effect()], , effect()];
}

export function test2() {
  function f () {
    const a = {};
    const b = {};
    return [a, b];
  }
  const [x, y] = f();
  t = x;
}
