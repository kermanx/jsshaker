export function main() {
  import("a")
  import(effect(1) + "b", { c: effect(2) })
  effect(import("c", effect(3))) 
}
