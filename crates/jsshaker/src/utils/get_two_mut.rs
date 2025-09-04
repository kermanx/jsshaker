use oxc_index::{Idx, IndexVec};

pub fn get_two_mut_from_vec<K, V>(
  vec: &mut IndexVec<K, V>,
  index1: K,
  index2: K,
) -> (&mut V, &mut V)
where
  K: Idx,
{
  unsafe {
    let vec = vec as *mut IndexVec<K, V>;
    (&mut (&mut (*vec))[index1], &mut (&mut (*vec))[index2])
  }
}
