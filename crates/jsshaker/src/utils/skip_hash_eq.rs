pub struct SkipHashEq<T>(pub T);

impl<T> PartialEq for SkipHashEq<T> {
  fn eq(&self, _: &Self) -> bool {
    true
  }
}

impl<T> Eq for SkipHashEq<T> {}

impl<T> std::hash::Hash for SkipHashEq<T> {
  fn hash<H: std::hash::Hasher>(&self, _: &mut H) {}
}

impl<T> std::fmt::Debug for SkipHashEq<T>
where
  T: std::fmt::Debug,
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    self.0.fmt(f)
  }
}
