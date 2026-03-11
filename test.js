function f() {
  const o = {};
  o.prop = 1
  if (u) return o;
  o.prop = 2
  if (u) return o;
}

t = f()