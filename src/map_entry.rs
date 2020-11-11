use std::default::Default;
use std::hash::Hash;

type HashValue = usize;

#[derive(Clone, Copy, Debug)]
pub enum MapEntry<K: Hash + Eq, V> {
    Occupied(Entry<K, V>),
    VacantEntry,
}

impl<K: Hash + Eq, V> MapEntry<K, V> {
    /// Returns the contained `Occupied` map entry, consuming the self value.
    /// This function will panic if you try to unwrap a `VacantEntry`.
    pub fn unwrap(self) -> Entry<K, V> {
        if let MapEntry::Occupied(entry) = self {
            return entry;
        } else {
            panic!("Expected an Occupied entry (non-vacant MapEntry) instead found a VacantEntry");
        }
    }
}

impl<K: Hash + Eq, V> Default for MapEntry<K, V> {
    fn default() -> Self {
        MapEntry::VacantEntry
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Entry<K: Hash + Eq, V> {
    pub key: K,
    pub value: V,
    pub hash: HashValue,
    /// The probe sequence length. The PSL of an entry is the number of probes required to find the key during lookup.
    pub psl: usize,
}

impl<K: Hash + Eq, V> Entry<K, V> {
    pub fn new(key: K, value: V, hash: usize, psl: usize) -> Self {
        Self {
            key,
            value,
            hash,
            psl,
        }
    }
}
