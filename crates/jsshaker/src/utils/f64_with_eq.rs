use std::hash::{Hash, Hasher};

#[derive(Debug, Copy, Clone)]
pub struct F64WithEq(pub f64);

impl PartialEq<Self> for F64WithEq {
  fn eq(&self, rhs: &Self) -> bool {
    self.0.to_le_bytes() == rhs.0.to_le_bytes()
  }
}

impl Hash for F64WithEq {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.0.to_le_bytes().hash(state)
  }
}

impl From<f64> for F64WithEq {
  fn from(val: f64) -> Self {
    Self(val)
  }
}

impl From<F64WithEq> for f64 {
  fn from(val: F64WithEq) -> Self {
    val.0
  }
}

impl Eq for F64WithEq {}
