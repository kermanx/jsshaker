export function main() {
  while(false) {
    0
  }
  
  while(false) {
    effect
  }
  
  while(false) {
    let a = 0;
    a++;
  }
  
  while((effect, false)) {
    effect
  }
  
  while(sth) {
    effect1;
    break;
    effect2;
  }

  while(sth) {
    effect1;
    continue;
    effect2;
  }

  while(sth) {
    effect1;
    return;
    effect2;
  }

  while(sth1) {
    effect1;
    while(sth2) {
      effect2;
      break;
      effect3;
    }
    effect2;
  }

  while(1) {
    effect1;
    return;
    effect2;
  }

  outer: while(a) {
    effect1;
    while(b) {
      effect2;
      break outer;
    }
    effect3;
  }

  while(sth()) {
    if (a) {
      break;
    }
    else {
      if (b) {
        break;
      }
      else {
        continue;
      }
      effect1;
    }
    effect2;
  }
  
  while(sth()) {
    i: if (a) {
      break i;
    }
    else {
      if (b) {
        break;
      }
      else {
        continue;
      }
      effect1;
    }
    effect2;
  }
}
