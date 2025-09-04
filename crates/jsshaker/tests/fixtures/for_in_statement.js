export function main(a) {
  for(let k in a) { }
  for(let k in g) { }
  for(g in a) { }

  for(let k in a) { effect() }

  for(let k in a) { effect(k) }

  const pure = () => a
  for(let k in { x: 1, y: pure() }) { effect(k) }
}
