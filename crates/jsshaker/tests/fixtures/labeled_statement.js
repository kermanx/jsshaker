export function main() {
  a: b: {
    effect1();
    c: {
      effect2();
      break c;
      effect3();
    }
    effect4();
    {
      effect5();
      break a;
      effect6();
    }
    effect7();
  }
}
