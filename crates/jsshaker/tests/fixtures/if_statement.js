export function truthy() {
  s1 ; if (1) {2} else {3}
  s2 ; if (1) {e} else {3}
  s3 ; if (1) {2} else {e}
  s4 ; if (1) {e} else {f}

  s5 ; if (1) {2}
  s6 ; if (1) {e}

  s7 ; if ((e,1)) {2} else {3}
  s8 ; if ((e,1)) {e} else {3}
  s9 ; if ((e,1)) {2} else {e}
  s10; if ((e,1)) {e} else {f}

  s11; if ((e,1)) {2}
  s12; if ((e,1)) {e}
}

export function falsy() {
  s1 ; if (0) {2} else {3}
  s2 ; if (0) {e} else {3}
  s3 ; if (0) {2} else {e}
  s4 ; if (0) {e} else {f}

  s5 ; if (0) {2}
  s6 ; if (0) {e}
  
  s7 ; if ((e,0)) {2} else {3}
  s8 ; if ((e,0)) {e} else {3}
  s9 ; if ((e,0)) {2} else {e}
  s10; if ((e,0)) {e} else {f}

  s11; if ((e,0)) {2}
  s12; if ((e,0)) {e}
}

export function unknown(a) {
  s1 ; if (a) {2} else {3}
  s2 ; if (a) {e} else {3}
  s3 ; if (a) {2} else {e}
  s4 ; if (a) {e} else {f}

  s5 ; if (a) {2}
  s6 ; if (a) {e}

  s7 ; if (e) {2} else {3}
  s8 ; if (e) {e} else {3}
  s9 ; if (e) {2} else {e}
  s10; if (e) {e} else {f}

  s11; if (e) {2}
  s12; if (e) {e}
}

export function with_effect(unknown) {
  function f(e) {
    if (e) return;
    effect();
  }
  f(unknown)

  let a = {}
  if (a) {
    e1
  } else {
    e2
  }
}

export function with_effect_2() {
  function f(t) {
    effect()
    if(t) effect()
  }
  f(0)
  f(1)
}

export function with_effect_3() {
  function f(t) {
    if(t) effect()
  }
  f(0)
  f(1)
}
