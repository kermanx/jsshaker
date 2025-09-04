export function pure() {
  t1 = 1 && 2;
  t2 = 0 && 'a';
  t3 = 1 || 2;
  t4 = 0 || 'a';
  t5 = 1 ?? 2;
  t6 = 0 ?? 'a';
  t7 = null ?? 2;
}

export function impure(a) {
  a && effect;
  a || effect;
  
  (effect, 1) && effect1;
  (effect, 0) && effect2;
  (effect, 1) || effect3;
  (effect, 0) || effect4;

  t1 = (effect1, 1) && effect1;
  t2 = (effect2, 0) && effect2;
  t3 = (effect3, 1) || effect3;
  t4 = (effect4, 0) || effect4;

  function f(b) {
    b && effect;
  }
  f(0);
  f(1);

  
  function g(b) {
    b || effect;
  }
  g(0);
  g(1);

  function h(a) {
    effect()
    return a && effect(a)
  }
  h(false)
  h({})  
}
