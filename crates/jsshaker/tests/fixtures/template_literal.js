export function main(a) {
  test1(`a${a}b`);

  test2(`a${3+4}b${a}${6}d`)

  test3(`a${(effect,2)}b${(effect,3)}`)

  test4(`a${(effect,2)}b${3}c${a}`)

  test5(`a${(effect,2)}b${a}c${3}`)
  
  test6(`a${(effect,2)}b${a}c${(effect, 3)}`)

  test7(`\\${a}\`${b}\n${c}\${}`)
}
