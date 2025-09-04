# Object Property Mangling

## 背景

JS 压缩器已经能够完美地删除代码中的无用字符，如空格和一些分号。同时它也能重命名变量名，比如：

```js
const myVariable = { value: 42 };
log(myVariable.value * 2);
```

会被压缩为：

```js
let e={value:42};log(e.value*2)
```

但是它无法处理对象属性名的重命名，如上例中的 `value`。此类属性名由于 JS 的动态特性，很难被分析和重命名。而在压缩后的代码中，这些属性名占据可观的空间，尤其是在 gzip 之后。这是 JS 代码体积优化中重要的未解决难题。

通过一些简单的估计，如果此项优化达到完美，可以节约约 30% 的代码体积，无论对于库还是应用。（需要补充数据）

## 现有方案

### Terser 等

Terser 支持对象属性名的重命名，但从设计上无法保证安全，因此很难被实际使用。比如如下情况：

```js
const obj = { foo: v1, bar: v2 };
const key = t1 ? "foo" : "bar";
log(obj.foo, obj[key]);
```

会被压缩为：

```js
const obj = { a: v1, b: v2 };
const key = t1 ? "foo" : "bar";
log(obj.a, obj[key]);
```

而后者在显然是错误的。这是由于它没有办法分析动态访问属性的情况。这是基于规则的优化器的局限性，因为规则不可能编写得无限复杂，而现实中的情况无穷无尽，为了效果好，它只能牺牲正确性。

### esbuild 等

它们采取了一种变通方式。它们通过允许用户手动配置优化的白名单/黑名单，比如只优化 `_` 开头的属性名等。这种方式缺少通用性，且配置起来相对麻烦，把风险留给了用户。

### VSCode 的打包器

VSCode 使用 TypeScript 编写，它在打包时通过调用 TypeScript 提供的代码重构功能来重命名属性名，实现相对安全的属性压缩。这种方式的缺点是只能用于 TypeScript 项目，且项目中若存在跳过类型检查的行为（比如 `as any`），则会导致结果错误。且无法通过此方法压缩打包的第三方库的代码。不适合一般应用程序使用。其[官方博客](https://code.visualstudio.com/blogs/2023/07/20/mangling-vscode)指出，这种方式减小了 14% 的体积。

## 实现

该优化在 JsShaker 基本的分析框架下完成。

### 字符串字面量的重命名

首先，JsShaker 实现了字符串字面量做比较的重命名，即：

```js
const s1 = "hello", s2 = unknown ? "hello" : "world";
log(s1 === s2);
```

优化为：

```js
const s1 = "a", s2 = unknown ? "a" : "b";
log(s1 === s2);
```

以下是优化的原理：

#### 不可重命名的约束

每当碰到可以折叠为字符串的常量，就为它的值会带有一个 “无法被重命名” 的约束（下文称为 `NoMangle(node)`）。比如：

```js
const str = "hello";
log(str);
```

- 分析完第一行后，`str` 的值是 `"hello"`，依赖于 `NoMangle("hello")`。
- 第二行中，`log` 函数调用会褪优化作为参数的 `str`，于是 `"hello"` 节点就被标记为不可重命名了。

因此，这个优化得到的结果是 `"hello"` 无法被重命名。这是符合预期的。

#### 相等性的约束

当数个字符串之间只做等于和不等于比较时，它们就可以被重命名。这是通过相等性的约束来实现的。比如上文的例子：

```js
const s1 = "hello", s2 = unknown ? "hello" : "world";
//         ^ A                     ^ B       ^ C
log(s1 === s2);
```

在 `s1 === s2` 处，`s1` 的值是 `"hello"`，依赖于 `NoMangle(A)`。而 `s2` 的值是 `"hello"` 或者 `"world"`，分别依赖于 `NoMangle(B)` 和 `NoMangle(C)`。

`s1 === s2` 运算的结果是 `true` 或者 `false`，分别依赖于 `"hello" === "hello"` 和 `"hello" !== "world"` 的事实。因此，该运算的结果被分析为 `true` 依赖于 `Equal(A, B)`，`false` 依赖于 `Unequal(A, C)`。而之前的 `NoMangle(x)` 依赖会被抛弃，因为它已不再是值的依赖。

`Equal` 和 `Unequal` 约束也可以包含两个以上的节点。表示它们之间的值是相等的或两两不同。

#### 约束的求解

通过上述方式，在分析完成后，我们会得到一组约束，包含 `NoMangle`，`Equal` 和 `Unequal`。简单计算就可以得到各个相关节点满足约束的最短值是什么。计算过程可以写在论文里，但我还不会怎么写成伪代码。

### 对象属性的重命名

以上内容实现了字符串字面量的重命名，为对象属性的重命名铺平了道路。

#### 转化为字符串字面量的重命名

首先我们把静态属性访问当作动态属性访问来处理，显式表达出来，所有对象相关的语法都是字符串字面量相关的操作：

```js
({ foo: "hello", bar: "world" })
// --> { "foo": "hello", "bar": "world" }
obj.foo
// --> obj["foo"]
```

#### 属性名不能冲突

每个对象之间的属性名称不能冲突。我们使用 `Unequal` 约束来表达：

```js
const obj = { "foo": 1, "bar": 2, "baz": 3 };
//            ^A        ^B        ^C
```

此时 `obj` 里的各个值都依赖于 `Unequal(A, B, C)`。

#### 访问存在的属性

访问对象时，若确实存在这个属性，则返回的值依赖于 `Equal` 约束。比如：

```js
const obj = { "foo": 1, "bar": 2 };
//            ^A        ^B
const key = "foo";
//          ^C
x = obj[key]
```

在这里，`obj[key]` 的值 `1` 依赖于 `Equal(A, C)` 和 `Unequal(A, B)`，但不依赖于 `NoMangle(A)` 和 `NoMangle(C)`。

#### 访问不存在的属性

访问对象时，若不存在这个属性，则返回的值依赖于 `Unequal` 约束。比如：

```js
const obj = { "foo": 1, "bar": 2 };
//            ^A        ^B
const key = "baz";
//          ^C
x = obj[key]
```

在这里，`obj[key]` 的值 `undefined` 依赖于 `Unequal(A, B, C)`。

#### 访问动态属性

```js
const obj = { "foo": 1, "bar": 2 };
//            ^A        ^B
const key = unknown ? "foo" : "baz";
//                    ^C      ^D
x = obj[key]
```

在这里，`obj[key]` 的值可能是：

- `1`，依赖于 `Equal(A, C)` 和 `Unequal(A, B)`；
- `undefined`，依赖于 `Unequal(A, B, D)`；

求解这些约束，得出 `A` 最小可以是 `"a"`，`B` 最小可以是 `"b"`，`C` 最小可以是 `"a"`，`D` 最小可以是 `"c"`。

#### 无法追踪的情况

除非整个环境的原型链被污染，本文的优化要求绝对安全。在碰到无法追踪的情况时，优化会放弃。比如：

```js
const obj = { "foo": 1, "bar": 2 };
//            ^A        ^B
x = obj[unknown]
```

由于 `unknown` 的值是未知的，因此 `obj[unknown]` 的值是未知值，依赖于 `NoMangle(A)` 和 `NoMangle(B)`。最终生成的约束也即这两者，结果就是无法优化属性名。

#### 原型链上的值

通过模拟 JS 访问原型链的逻辑，生成多组 `Equal` 和 `Unequal` 约束即可。不再赘述。

## 效果

首次实现了 JavaScript 的完全安全的对象属性名重命名。

## 不足

受制于整个基本的分析框架的限制，许多动态访问的属性名的值无法被精确地得出，因此无法开展对象属性名的重命名，使得实际效果大打折扣。在这个情况下，此项优化率往往只能贡献可以忽略不计的体积优化。
