// https://github.com/KermanX/jsshaker/issues/62
function* bar() {
  yield 4;
  console.log('Hi');
  return 5;
}
for (const foo of bar()) {
  if (foo === 4) break;
}

export function unconditional_break(it) {
  for (const x of it()) {
    break;
  }
}

export function conditional_return(it) {
  for (const x of it()) {
    if (x) return;
  }
}

export function conditional_throw(it) {
  for (const x of it()) {
    if (x) throw x;
  }
}

// No early exit: should still be folded into a spread.
export function no_early_exit(it) {
  for (const x of it()) {
  }
}

// `continue` does not prevent exhausting the iterator: should still fold.
export function only_continue(it) {
  for (const x of it()) {
    if (x) continue;
  }
}

// Side effect before the break: the break still prevents exhaustion.
export function effect_before_break(it, x) {
  for (const v of it()) {
    console.log(v);
    if (x) break;
  }
}

// Labeled break out of the loop from inside a switch.
export function labeled_switch_break(it, x) {
  loop: for (const v of it()) {
    switch (v) {
      case 1:
        if (x) break loop;
    }
  }
}

// An uncaught throw escapes the loop.
export function throw_in_loop(it, x) {
  for (const v of it()) {
    if (x) throw v;
  }
}

export async function for_await_break(it, x) {
  for await (const v of it()) {
    if (x) break;
  }
}

// The break exits the loop from inside a try/finally.
export function break_in_try_in_loop(it, x) {
  for (const v of it()) {
    try {
      if (x) break;
    } finally {
      console.log('f');
    }
  }
}

// Labeled continue to the outer loop passes through a try (not a throw).
export function continue_through_try(xs, x) {
  outer: for (const a of xs) {
    for (const b of xs) {
      try {
        if (x) continue outer;
      } finally {
        console.log('f');
      }
    }
  }
}
