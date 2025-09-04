function f(a) {
  return function (b) {
    a = a + b
    return a
  }
}
    
test(f(2)(3))

const x = f(10)
test(x(5))
test(x(7))
