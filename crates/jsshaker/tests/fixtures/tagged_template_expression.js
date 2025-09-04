export function main(a) {
  let pure = (x) => 1;
  pure`a`;
  pure`b${effect(2)}c`;

  effect(pure`a${3}`);

  let impure = (x) => effect(x);
  impure`a`;  
  impure`b${effect()}c`;
}
