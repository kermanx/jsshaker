#[macro_export]
macro_rules! define_ptr_idx {
  ($v:vis struct $type:ident for $target:ty;) => {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    $v struct $type {
      ptr: std::ptr::NonNull<u8>,
    }

    impl $type {
      #[inline(always)]
      fn from_ref<'a>(r: &$target) -> Self {
        Self { ptr: std::ptr::NonNull::from(r).cast() }
      }
      #[inline(always)]
      pub fn as_ref<'a>(&self) -> &'a $target {
        unsafe { self.ptr.cast().as_ref() }
      }
    }
  };
}
