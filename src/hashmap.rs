use super::fx_build_hasher::FxBuildHasher;
use super::map_entry::{Entry, MapEntry};
use std::hash::{BuildHasher, Hash, Hasher};

const INITIAL_SIZE: usize = 4;

// TODO: Complete robinhood implementation.

/// Robinhood HashMap backed by the fx hashing algorithm.
#[derive(Debug)]
pub struct FxHashMap<K: Hash + Eq, V, H: BuildHasher + Clone> {
    inner: Vec<MapEntry<K, V>>,
    hasher_builder: H,
    num_items: usize,
}

impl<K: Hash + Eq, V> FxHashMap<K, V, FxBuildHasher> {
    /// Creates a `FxHashMap` with the default Fx Hasher and an initial capacity of 0.
    pub fn new() -> Self {
        let hasher_builder = FxBuildHasher::new();

        Self {
            inner: Vec::new(),
            hasher_builder,
            num_items: 0,
        }
    }

    /// Constructs a `FxHashMap` with an initial capacity. This method of constructing is recommended if you have a good idea of how large
    /// your hashmap will grow as this reduces the number of resizes.
    pub fn with_capacity(initial_capacity: usize) -> Self {
        let hasher_builder = FxBuildHasher::new();
        let mut inner: Vec<MapEntry<K, V>> = Vec::with_capacity(initial_capacity);
        inner.extend((0..initial_capacity).map(|_| MapEntry::default()));

        Self {
            inner,
            hasher_builder,
            num_items: 0,
        }
    }
}

impl<K: Hash + Eq, V, H: BuildHasher + Clone> FxHashMap<K, V, H> {
    /// Creates a `FxHashMap` with a custom hasher builder which overrides the default fx hasher. Use this if you want to create a
    /// robinhood hashmap but with a custom hasher perhaps to provide greater cryptographic security.
    pub fn with_hasher(hasher_builder: H) -> Self {
        Self {
            inner: Vec::new(),
            hasher_builder,
            num_items: 0,
        }
    }

    /// Creates a `FxHashMap` with both an initial capacity and a custom hasher.
    pub fn with_capacity_and_hasher(initial_capacity: usize, hasher_builder: H) -> Self {
        let mut map = FxHashMap::with_hasher(hasher_builder);
        let mut inner: Vec<MapEntry<K, V>> = Vec::with_capacity(initial_capacity);
        inner.extend((0..initial_capacity).map(|_| MapEntry::default()));
        map.inner = inner;

        map
    }

    /// Inserts a value with its associated key into the hashmap. Time complexity should be amortized O(1).
    pub fn insert(&mut self, key: K, value: V) {
        // Load Factor of 0.75 (can be upped to 0.85 or so once robinhood implementation is complete)
        if self.inner.is_empty() || self.num_items > 3 * self.inner.len() / 4 {
            self.resize();
        }

        let hash = self.hash_key(&key);
        // Handles insertion logic
        self.insert_with_hash(key, value, hash);
    }

    fn insert_with_hash(&mut self, key: K, value: V, hash: usize) {
        let slot = hash % self.inner.len();

        let spot = self.inner.get_mut(slot).unwrap();
        // If none exists at the required slot then we'll simply just insert into that slot.
        if let MapEntry::VacantEntry = spot {
            let _ = std::mem::replace(spot, MapEntry::Occupied(Entry::new(key, value, hash, 0)));
            return self.num_items += 1;
        } else {
            // Conflict. We'll try to resolve this conflict via a FCFS (first come first serve) approach.
            // That is, the first entry to come at the required slot will remain there, while all later entries will simply start
            // walking until they find an empty spot.
            // In the future we'll use the robinhood method to decrease variance.

            let mut i = slot;

            // Start walking until we find an empty spot.
            while i < self.inner.len() {
                let cur = self.inner.get_mut(i).unwrap();
                if let MapEntry::Occupied(entry) = cur {
                    if *entry.get_key() == key {
                        // Update value
                        let _ = std::mem::replace(entry, Entry::new(key, value, hash, i - slot));
                        return;
                    }

                    i += 1;
                } else {
                    // Insert entry into the vacancy.
                    let _ = std::mem::replace(
                        cur,
                        MapEntry::Occupied(Entry::new(key, value, hash, i - slot)),
                    );
                    return self.num_items += 1;
                }
            }

            // Our probing has reached the end of the inner vector. We'll just push the entry to the back of the vector.
            self.inner
                .push(MapEntry::Occupied(Entry::new(key, value, hash, i - slot)));
            return self.num_items += 1;
        }
    }

    /// Gets the appropriate value given a valid key. Returns `None` if the key value mapping does not exist.
    /// NOTE: Current implementation is somewhat inefficient in the case of failed lookups since we would just probe until the end of
    /// the backing vector. Ideally we should be storing the max PSL recorded so that we can smartly decide when to stop the probing.
    pub fn get(&self, key: &K) -> Option<&V> {
        let hash = self.hash_key(key);
        let slot = hash % self.inner.len();
        let mut i = slot;

        while i < self.inner.len() {
            let cur = self.inner.get(i).unwrap();
            if let MapEntry::Occupied(entry) = cur {
                if *entry.get_key() == *key {
                    return Some(entry.get_value());
                }
            } else {
                return None;
            }

            i += 1;
        }

        return None;
    }

    /// Gets the size / number of entries of the hashmap.
    pub fn size(&self) -> usize {
        self.num_items
    }

    /// Gets the capacity of the hashmap.
    pub fn capacity(&self) -> usize {
        self.inner.len()
    }

    fn resize(&mut self) {
        let target_size: usize = match self.inner.len() {
            0 => INITIAL_SIZE,
            n => 2 * n,
        };

        let mut new_map = Self::with_capacity_and_hasher(target_size, self.hasher_builder.clone());
        // Filters out all vacant entries since we don't care about those.
        let entries = self.inner.drain(0..).filter_map(|entry| {
            if let MapEntry::Occupied(inner_entry) = entry {
                return Some(inner_entry);
            } else {
                return None;
            }
        });

        for entry in entries {
            // Transfer ownership to the new hashmap.
            let (key, value, hash) = entry.get_as_owned();
            // No need of recomputing hashes again.
            new_map.insert_with_hash(key, value, hash);
        }

        // Replace with the new resized hashmap.
        let _ = std::mem::replace(self, new_map);
    }

    /// Builds a new hasher, hashes the provided key and returns the hash.
    fn hash_key(&self, key: &K) -> usize {
        let mut hasher = self.hasher_builder.build_hasher();
        key.hash(&mut hasher);
        hasher.finish() as usize
    }
}

#[cfg(test)]
mod tests {
    use super::super::fx_build_hasher::FxBuildHasher;
    use super::*;

    #[test]
    fn it_constructs_with_an_initial_capacity() {
        let initial_capacity = 5;
        let hashmap: FxHashMap<&str, i32, FxBuildHasher> =
            FxHashMap::with_capacity(initial_capacity);

        assert_eq!(hashmap.capacity(), initial_capacity);
    }

    #[test]
    fn it_inserts_values_without_initial_capacity() {
        let mut hashmap: FxHashMap<&str, i32, FxBuildHasher> = FxHashMap::new();
        let value_to_insert: i32 = 123;

        hashmap.insert("hello", value_to_insert);
        assert_eq!(*hashmap.get(&"hello").unwrap(), value_to_insert);
    }

    #[test]
    fn it_inserts_values_with_initial_capacity() {
        let mut book_reviews = FxHashMap::with_capacity(10);
        let key = "The Adventures of Sherlock Holmes".to_string();
        let value = "Eye lyked it alot.".to_string();

        book_reviews.insert(key, value);

        assert_eq!(book_reviews.capacity(), 10);
        assert_eq!(
            *book_reviews
                .get(&String::from("The Adventures of Sherlock Holmes"))
                .unwrap(),
            String::from("Eye lyked it alot.")
        );
    }
}
