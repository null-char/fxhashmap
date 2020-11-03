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
    key: K,
    value: V,
    hash: HashValue,
    /// The probe sequence length. In this context it is the amount of distance the entry is from relative to its initial slot (hash mod len).
    psl: usize,
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

    pub fn get_as_owned(self) -> (K, V, usize) {
        return (self.key, self.value, self.hash);
    }

    pub fn get_key(&self) -> &K {
        &self.key
    }

    pub fn get_value(&self) -> &V {
        &self.value
    }

    pub fn get_hash(&self) -> HashValue {
        self.hash
    }
}
