use oxc::allocator;

use crate::scope::rw_tracking::ReadWriteTarget;

#[derive(Debug)]
pub struct FnCacheTrackingData<'a> {
  pub outer_deps: allocator::HashSet<'a, ReadWriteTarget<'a>>,
}
