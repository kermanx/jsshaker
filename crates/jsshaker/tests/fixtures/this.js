const obj1 = {
  getThis() {
    return this;
  }
}
obj1.getThis()
test1(obj1.getThis())
test1((0, obj1).getThis())
test1((0, obj1.getThis)())
test1(((((obj1.getThis))))())

const obj2 = {
  getThis() {
    return this ? 1 : 2;
  }
}
obj2.getThis()
test2(obj2.getThis())
test2((0, obj2).getThis())
test2(((((obj2.getThis))))())
test2((0, obj2.getThis)())
