// @minify

export function f() {
  function g(a) {
    if (a)
      console.log('effect')
    else
      return 'str'
  }
  let { ["x"]: y = 1 } = { x: g('') ? undefined : g(1) }
  return y
}
