export function main() {
  for(let i = 0; i < 10; i++) {
    // Nothing
  }

  for(effect(1);(effect(2), false);effect(3)){
    effect(4);
  }

  for(let i = 0 + 1; test(i); i+=1+1){
    effect(5);
  }

  let i = 1;
  for(i = 0;test(i);i++) {
    effect(6);
  }
  print(i);

  for(let i = 0;test(i);i++) {
    effect(7);
    break;
    effect(8);
  }

  outer: for(let i = 0;test(i);i++) {
    effect(9);
    for(let j = 0;test(j);j++) {
      effect(10);
      break outer;
      effect(11);
    }
    effect(10);
  }
}
