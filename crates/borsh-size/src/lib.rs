/// This file defines a trait for computing the borsh-serialized size of common
/// types.
///
// This isn't the same as the version in bonfida-utils, which assumes all
// elements of Vec<T> have the same borsh size.
use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    StorageUsage,
};
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

/// The overhead to store a string with Borsh. Borsh serializes Strings as
/// byte slices. Byte slices are serialized with a size prefix, followed by
/// the bytes.
///
/// [size:u32, u8, u8, u8, ...]
///
/// <https://docs.rs/borsh/latest/src/borsh/ser/mod.rs.html#200>
pub const STRING_OVERHEAD: StorageUsage = 4;

/// The overhead to store a HashMap with Borsh. Borsh serializes HashMap<K,
/// V> with a size prefix, followed by Borsh serialization of (K, V) pairs
/// (keys are sorted before writing):
///
/// [size:u32, K1, V1, K2, V2, ...]
///
/// <https://docs.rs/borsh/latest/src/borsh/ser/mod.rs.html#314>
pub const HASH_MAP_OVERHEAD: StorageUsage = 4;

/// The overhead to store a HashSet with Borsh. Borsh serializes HashSet<T>
/// with a size prefix, followed by the Borsh serialization of whatever is
/// inside (items are sorted before writing).
///
/// [size:u32, T, T, T, ...]
///
/// <https://docs.rs/borsh/latest/src/borsh/ser/mod.rs.html#334>
pub const HASH_SET_OVERHEAD: StorageUsage = 4;

/// The overhead to store a Vec with Borsh. Borsh serializes Vec<T> with a
/// size prefix, followed by the Borsh serialization of whatever is inside.
///
/// [size:u32, T, T, T, ...]
///
/// <https://docs.rs/borsh/latest/src/borsh/ser/mod.rs.html#200>
pub const VEC_OVERHEAD: StorageUsage = 4;

pub trait BorshSize: BorshDeserialize + BorshSerialize {
    fn borsh_size(&self) -> StorageUsage;
}

impl BorshSize for u64 {
    fn borsh_size(&self) -> StorageUsage {
        8
    }
}

impl BorshSize for u128 {
    fn borsh_size(&self) -> StorageUsage {
        16
    }
}

impl BorshSize for String {
    fn borsh_size(&self) -> StorageUsage {
        STRING_OVERHEAD + self.len() as u64
    }
}

impl<T: BorshSize> BorshSize for Vec<T> {
    fn borsh_size(&self) -> StorageUsage {
        if self.is_empty() {
            VEC_OVERHEAD
        } else {
            VEC_OVERHEAD + self.iter().map(|v| v.borsh_size()).sum::<u64>()
        }
    }
}

impl<K, V> BorshSize for HashMap<K, V>
where
    K: Eq + PartialOrd + Hash + BorshSize,
    V: BorshSize,
{
    fn borsh_size(&self) -> StorageUsage {
        if self.is_empty() {
            HASH_MAP_OVERHEAD
        } else {
            HASH_MAP_OVERHEAD
                + self
                    .iter()
                    .map(|(k, v)| k.borsh_size() + v.borsh_size())
                    .sum::<u64>()
        }
    }
}

impl<T> BorshSize for HashSet<T>
where
    T: Eq + PartialOrd + Hash + BorshSize,
{
    fn borsh_size(&self) -> StorageUsage {
        if self.is_empty() {
            HASH_SET_OVERHEAD
        } else {
            HASH_SET_OVERHEAD + self.iter().map(|v| v.borsh_size()).sum::<u64>()
        }
    }
}
