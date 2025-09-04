effect(f(1), x)

function f(a) {
  effect(2);
  return a;
}

export var x = 3;
