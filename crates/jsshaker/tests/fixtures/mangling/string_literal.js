export function test1() {
  function f(x) {
    if (x === "foo") {
      console.log("x is foo");
    } else {
      console.log("x is bar");
    }
  }
  f("foo")
  f("bar")
}

export function test2(unknown) {
  const key = unknown ? "foo" : "bar";
  if (key === "foo") {
    console.log("key is foo");
  } else if (key === "bar") {
    console.log("key is bar");
  } else {
    console.log("unreachable");
  }
}
