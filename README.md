# JsShaker

This is an experimental code size optimizer for JavaScript based on [the Oxc parser](https://oxc.rs).

[**Try online**](https://kermanx.com/jsshaker) | [**CLI**](https://www.npmjs.com/package/jsshaker) | [**Bundler Plugin**](https://www.npmjs.com/package/rollup-plugin-jsshaker)

- Up to 30% size reduction on real-world apps (compared to Rollup)
- Supports latest JavaScript features

## Examples

### Constant Folding

<table><tbody><tr><td width="500px"> Input </td><td width="500px"> Output </td></tr><tr>
<td valign="top">

```js
export function f() {
  function g(a) {
    if (a) console.log("effect");
    else return "str";
  }
  let { ["x"]: y = 1 } = { x: g("") ? undefined : g(1) };
  return y;
}
```

</td><td valign="top">

```js
export function f() {
  return 1;
}
```

</td></tr></tbody></table>

### Remove Dead Code

> The core of tree-shaking. The execution is simulated to know which code is useless.
>
> And don't worry about the `&& true` in the output, minifier will remove it.

<table><tbody><tr><td width="500px"> Input </td><td width="500px"> Output </td></tr><tr>
<td valign="top">

```js
function f(value) {
  if (value) console.log(`${value} is truthy`);
}
f(1);
f(0);

function g(t1, t2) {
  if (t1 && t2) console.log(2);
  else if (t1 || t2) console.log(1);
  else console.log(0);
}
g(true, true);
g(false, false);
```

</td><td valign="top">

```js
function f() {
  {
    console.log("1 is truthy");
  }
}
f();

function g(t1) {
  if (t1 && true) console.log(2);
  else {
    console.log(0);
  }
}
g(true);
g(false);
```

</td></tr></tbody></table>

### Object Property Mangling

> This is beyond the scope of tree-shaking, we need a new name for this project 😇.

<table><tbody><tr><td width="500px"> Input </td><td width="500px"> Output </td></tr><tr>
<td valign="top">

```js
export function main() {
  const obj = {
    foo: v1,
    [t1 ? "bar" : "baz"]: v2,
  };
  const key = t2 ? "foo" : "bar";
  console.log(obj[key]);
}
```

</td><td valign="top">

```js
export function main() {
  const obj = {
    a: v1,
    [t1 ? "b" : "c"]: v2,
  };
  const key = t2 ? "a" : "b";
  console.log(obj[key]);
}
```

</td></tr></tbody></table>

### Class Tree Shaking

> One of the hardest but the coolest.

<table><tbody><tr><td width="500px"> Input </td><td width="500px"> Output </td></tr><tr>
<td valign="top">

```js
class A {
  method(x) {
    console.log("A", x);
  }
  static static_prop = unknown;
}
class B extends A {
  method(x) {
    console.log("B", x);
  }
  unused() {
    console.log("unused");
  }
}
new B().method(A.static_prop);
```

</td><td valign="top">

```js
class A {
  static a = unknown;
}
class B extends A {
  a(x) {
    console.log("B", x);
  }
}
new B().a(A.a);
```

</td></tr></tbody></table>

### JSX

> `createElement` also works, if it is directly imported from `react`.

<table><tbody><tr><td width="500px"> Input </td><td width="500px"> Output </td></tr><tr>
<td valign="top">

```jsx
function Name({ name, info }) {
  return (
    <span>
      {name}
      {info && <sub> Lots of things never rendered </sub>}
    </span>
  );
}
export function Main() {
  return <Name name={"world"} />;
}
```

</td><td valign="top">

```jsx
function Name() {
  return (
    <span>
      {"world"}
      {}
    </span>
  );
}
export function Main() {
  return <Name />;
}
```

</td></tr></tbody></table>

### React.js

> We also have special handling for some React.js APIs. For example, React Context, `memo`, `forwardRef`, `useMemo`, etc.

<table><tbody><tr><td width="500px"> Input </td><td width="500px"> Output </td></tr><tr>
<td valign="top">

```jsx
import React from "react";
const MyContext = React.createContext("default");
function Inner() {
  const value = React.useContext(MyContext);
  return <div>{value}</div>;
}
export function main() {
  return (
    <MyContext.Provider value="hello">
      <Inner />
    </MyContext.Provider>
  );
}
```

</td><td valign="top">

```jsx
import React from "react";
const MyContext = React.createContext();
function Inner() {
  return <div>{"hello"}</div>;
}
export function main() {
  return (
    <MyContext.Provider>
      <Inner />
    </MyContext.Provider>
  );
}
```

</td></tr></tbody></table>

## Comparison

- **Rollup**: Rollup tree-shakes the code in a multi-module context, and it has information about the side effects of the modules. This project does a more fine-grained tree-shaking, and it can be used as a post-processor for Rollup, and is expected to produce smaller code.
- **Closure Compiler**/**swc**: they support both minification and dead code elimination, while this project is focused on tree-shaking (difference below). You can expect a size reduction when using JsShaker on their output, and vice versa.

### What's Tree Shaking?

Here is a simple comparison:

- Minification: Removing whitespace, renaming variables, syntax-level optimizations, etc.
- Dead code elimination: Removing code that is never executed, by using a set of rules, for example, "`if(false) { ... }` can be removed".
- Tree shaking: Removing code that is never executed, by simulating the runtime behavior of the code. For example, "`if (x) { ... }` can only be preserved if `...` is reachable and has side effects".

## Soundiness Statement

This project has been done in the spirit of soundiness. When building practical program analyses, it is often necessary to cut corners. In order to be open about language features that we do not support or support only partially, we are attaching this soundiness statement.

Our analysis does not have a fully sound handling of the following features:

- eval
- implicit conversions (==, valueOf, toString)
- exceptions and flow related to that
- prototype semantics

We have determined that the unsoundness in our handling of these features has minimal effect on analysis output and the validity of our experimental evaluation. To the best of our knowledge, our analysis has a sound handling of all language features other than those listed above.

This statement has been produced with the Soundiness Statement Generator from http://soundiness.org.
