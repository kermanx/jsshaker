export function f1(a) {
  effect1();
  throw (effect2(), a);
  effect3();
}

export function f2(a) {
  effect1();
  try {
    throw (effect2(), a);
  } catch (e) {
    effect3();
  }
  finally {
    effect4();
  }
}

export function f3(a) {
  try {
    1;
  } catch (e) {
    effect1();
  }
  finally {
    effect2();
  }
}

export function f4(a) {
  while (a) {
    try {
      effect1();
      break;
      effect2();
    }
    catch {}
    effect3();
  }
}
