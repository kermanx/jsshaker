export function main() {
  switch (2) {
    case 1:
      effect1();
    case 2:
      effect2();
    case 3:
      effect3();
  }

  switch (2) {
    default:
      effect1();
    case 1:
      effect2();
    case 2:
      effect3();
      break;
    case 3:
      effect4();
  }

  switch ("1" + unknown) {
    default:
      effect1();
    case 1:
      effect1();
    case "a":
      effect2();
      break;
    case 3:
      effect3();
  }

  switch ("1" + unknown) {
    case 1:
      1;2;3;
  }

  switch(unknown) {
    default:
    case 'a': console.log('a');
    case 'b': console.log('b');
  }
}