class A extends (1+1) {
  constructor(a, b) {
    super(a);
    b();
  }
  fn(a) {
    a;
    a = console.log;
    a();
  }
  [1+1] = 1+1
  get [3+3]() { return 3+3 }
  set x(v) { this._x = v }
}
new A()

export const a = class X { [1+1] = 1+1 };
export const b = class extends (1+1) { [1+1] = 1+1 };

export default class {
  a = 1;
}

class C {
  static a = effect(1);
  [effect(2)] = effect(3);
  static [effect(4)] = effect(5);
  static {
    effect(6);
  }
  [effect(7)]() {
    effect(8);
  }
}
