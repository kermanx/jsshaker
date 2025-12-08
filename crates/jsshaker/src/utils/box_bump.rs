use std::fmt::Debug;
use std::hash::Hash;
use std::ptr::NonNull;

use oxc::allocator::Allocator;

pub trait Idx: Debug + Clone + Copy + PartialEq + Eq + Hash {
  fn from_ptr(ptr: NonNull<u8>) -> Self;
  fn as_ptr(&self) -> NonNull<u8>;
}

#[macro_export]
macro_rules! define_box_bump_idx {
  ($v:vis struct $type:ident;) => {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    $v struct $type {
      ptr: std::ptr::NonNull<u8>,
    }

    impl $crate::utils::box_bump::Idx for $type {
      #[inline(always)]
      fn from_ptr(ptr: std::ptr::NonNull<u8>) -> Self {
        Self { ptr }
      }
      #[inline(always)]
      fn as_ptr(&self) -> std::ptr::NonNull<u8> {
        self.ptr
      }
    }
  };
}

pub struct BoxBump<'a, I: Idx, T> {
  allocator: &'a Allocator,
  _marker: std::marker::PhantomData<(I, T)>,
}

impl<'a, I: Idx, T> BoxBump<'a, I, T> {
  pub fn new(allocator: &'a Allocator) -> Self {
    BoxBump { allocator, _marker: std::marker::PhantomData }
  }

  #[inline(always)]
  pub fn alloc(&self, value: T) -> I {
    I::from_ptr(unsafe { NonNull::new_unchecked(self.allocator.alloc(value) as *mut _ as *mut u8) })
  }

  #[inline(always)]
  pub fn get(&self, idx: I) -> &T {
    unsafe { &*(idx.as_ptr().as_ptr() as *const T) }
  }

  #[inline(always)]
  pub fn get_mut(&mut self, idx: I) -> &mut T {
    unsafe { &mut *(idx.as_ptr().as_ptr() as *mut T) }
  }
}
