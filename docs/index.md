# JsShaker Implementation Notes

> Currently I am writing this in Chinese. Sorry for the inconvenience.

> 以下内容和实际实现在细节上有差别

## 基本思路

1. Analyzer: 模拟代码的执行过程，动态的将 AST 节点等单元（`DepId`）标记为“需要保留”。
2. Transformer: 根据 Analyzer 标记的需要保留的节点，生成新的 AST。

Transformer 的逻辑非常简单，速度也非常快。Analyzer 是相对复杂和慢的部分。

在 Analyzer 过程中，有两个重要的类型：

- Entity: 比如 `LiteralEntity::Number(1)`, `PrimitiveEntity::String`, `ObjectEntity` 等，用于追踪一个值的情况。不同于运行时的是，在 JsShaker 中我们往往不能获得确定的值，因此需要 `PrimitiveEntity::String` 表示未知的字符串，`UnionEntity` 表示可能为多个值中的一个，`UnknownEntity` 表示完全未知的值等。还有一类是 `ComputedEntity`，它用于给其他 Entity 附加一个 Consumable。
- Consumable：它可能是 `DepId`（可以标记为需要保留的最小单元），也可能是 entity，也可能是它们的组合。consumable 以一种统一的方式，打包了可以消耗/放弃追踪的元素。

在 Analyzer 过程中，有四种 Scope：

- CallScope: 调用栈
- CfScope: 控制流
- VariableScope: 变量作用域，由于有 oxc_semantic 的帮助，事实上一个函数调用才分配一个变量作用域
- TryScope: 抛出的异常在最近的 TryScope 被捕获

下面是一个简单的例子：

```js
globalVariable = 1 + (2, a);
```

Analyze 阶段，我们模拟运行时的执行流程，进行分析：

- 首先执行 AssignmentTarget (`globalVariable`) 的 RHS，即 BinaryExpression (`1 + (2, a)`)
  - 执行 LHS (`1`)，是 NumericLiteral，得到 `LiteralEntity::Number(1)`
  - 执行 RHS (`(2, a)`)，是 SequenceExpression，依次执行每个子表达式
    - 执行 `2`，得到 `LHS = LiteralEntity::Number(2)`
    - 执行 IdentifierReference (`a`)，此处 `a` 是未知变量，值是 `UnknownEntity`，并且基于 `DepId::IdentifierReference(a)`，因此得到 `RHS = ComputedEntity(UnknownEntity, DepId::IdentifierReference(a))`
  - 执行加法，由于 RHS 不是字面量，无法计算具体的值，故得到 `ComputedEntity(UnknownEntity, vec![LHS, RHS])`
- 执行赋值，由于赋值目标是未知的全局变量，因此无法追踪，所以直接消耗掉上一步计算出的值，并将 `DepId::SimplAssignmentTarget(globalVariable)` 标记为需要保留。

Transform 阶段：

- 转换 AssignmentExpression，由于 `DepId::SimplAssignmentTarget（globalVariable)` 被标记为需要保留，所以其 RHS 需要保留计算值 (即 `need_val` 参数为真)
- 转换 BinaryExpression，需要值，因此需要保留加号，且 LHS 和 RHS 都需要值
  - LHS 是 NumericLiteral，由于需要值，故直接保留
  - RHS 是 SequenceExpression，因此除最后一个表达式需要值外，前面的表达式都不需要值
    - 第一个表达式是 NumericLiteral，不需要值，直接删掉
    - 第二个表达式是 IdentifierReference，需要值，因此保留（其实此处其对应的节点也被标记为需要保留了）
    - 综合来看，只有第二个表达式转换后非空，因此只需返回第二个表达式即可
  - 根据转换后的 LHS 和 RHS，构造 BinaryExpression

## 逻辑模块

- [字面量内联](./literal-collector.md)
- [执行依赖](./execution-dep.md)
- [函数调用的分析](./function-call.md)
- [条件分支语句的分析](./conditional.md)
- _Work in progress..._
