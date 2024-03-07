pub mod hashmap;
pub mod hashset;

pub use hashmap::stacks_hash_map;
pub use hashset::stacks_hash_set;
use proptest::strategy::Strategy;

use crate::util::hash::Sha512Trunc256Sum;

pub fn sha_512_trunc_256_sum() -> impl Strategy<Value = Sha512Trunc256Sum> {
    (0..64u8).prop_map(|i| {
        let mut arr = [0u8; 64];
        arr[i as usize] = 1;
        Sha512Trunc256Sum::from_data(&arr)
    })
}
