pub struct SpanGuard {
  #[cfg(feature = "flame")]
  name: flame::StrCow,
  #[cfg(feature = "flame")]
  _guard: flame::SpanGuard,
}

#[cfg(feature = "flame")]
impl Drop for SpanGuard {
  fn drop(&mut self) {
    println!("- {}", self.name);
  }
}

impl SpanGuard {
  pub fn end(self) {}
}

#[cfg(feature = "flame")]
pub fn start_guard<S: Into<flame::StrCow>>(name: S) -> SpanGuard {
  let name = name.into();
  println!("+ {}", name);
  SpanGuard { name: name.clone(), _guard: flame::start_guard(name) }
}

#[cfg(not(feature = "flame"))]
pub fn start_guard<S: Into<String>>(_name: S) -> SpanGuard {
  SpanGuard {}
}
