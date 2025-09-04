export function main() {
  do {
    1;
  } while(0)

  do {
    effect(1);
  } while((effect(2), 0))

  let a = 1;
  do {
    a++;
  } while(a+1)
}
