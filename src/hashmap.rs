use super::fx_build_hasher::FxBuildHasher;
use super::map_entry::{Entry, MapEntry};
use std::{
    cmp::max,
    hash::{BuildHasher, Hash, Hasher},
    ptr,
};

const INITIAL_SIZE: usize = 4;

// TODO: Complete robinhood implementation.

/// Robinhood HashMap backed by the fx hashing algorithm (by default).
#[derive(Debug)]
pub struct RHMap<K: Hash + Eq, V, H: BuildHasher + Clone> {
    inner: Vec<MapEntry<K, V>>,
    hasher_builder: H,
    num_items: usize,
    max_psl: usize,
}

impl<K: Hash + Eq, V> RHMap<K, V, FxBuildHasher> {
    /// Creates a `RHMap` with the default Fx Hasher and an initial capacity of 0.
    pub fn new() -> Self {
        let hasher_builder = FxBuildHasher::new();

        Self {
            inner: Vec::new(),
            hasher_builder,
            num_items: 0,
            max_psl: 0,
        }
    }

    /// Constructs a `RHMap` with an initial capacity. This method of constructing is recommended if you have a good idea of how large
    /// your hashmap will grow as this reduces the number of resizes.
    pub fn with_capacity(initial_capacity: usize) -> Self {
        let hasher_builder = FxBuildHasher::new();
        let mut inner: Vec<MapEntry<K, V>> = Vec::with_capacity(initial_capacity);
        inner.extend((0..initial_capacity).map(|_| MapEntry::default()));

        Self {
            inner,
            hasher_builder,
            num_items: 0,
            max_psl: 0,
        }
    }
}

impl<K: Hash + Eq, V, H: BuildHasher + Clone> RHMap<K, V, H> {
    /// Creates a `RHMap` with a custom hasher builder which overrides the default fx hasher. Use this if you want to create a
    /// robinhood hashmap but with a custom hasher perhaps to provide greater cryptographic security.
    pub fn with_hasher(hasher_builder: H) -> Self {
        Self {
            inner: Vec::new(),
            hasher_builder,
            num_items: 0,
            max_psl: 0,
        }
    }

    /// Creates a `RHMap` with both an initial capacity and a custom hasher.
    pub fn with_capacity_and_hasher(initial_capacity: usize, hasher_builder: H) -> Self {
        let mut map = RHMap::with_hasher(hasher_builder);
        let mut inner: Vec<MapEntry<K, V>> = Vec::with_capacity(initial_capacity);
        inner.extend((0..initial_capacity).map(|_| MapEntry::default()));
        map.inner = inner;

        map
    }

    /// Inserts a value with its associated key into the hashmap. Time complexity should be amortized O(1).
    pub fn insert(&mut self, key: K, value: V) {
        // Load Factor of 0.75
        if self.inner.is_empty() || self.num_items > 3 * self.inner.len() / 4 {
            self.resize();
        }

        let hash = self.hash_key(&key);
        // Handles insertion logic
        self.insert_entry(Entry::new(key, value, hash, 0));
    }

    /// Deletes the entry with the given key. Returns an `Err` if no such entry with the provided key exists.
    pub fn remove(&mut self, key: &K) -> Result<(), &'static str> {
        // We're going to go with an interesting approach called backward shift deletion here
        let res = self.get_entry(&key);
        if let Some(entry) = res {
            let len = self.inner.len();
            let slot = entry.hash % len;
            // Index position of the entry to be deleted
            let i = slot + entry.psl;
            // To keep track of where the bucket ends so that we can shift all entries to the right of the entry
            // to be deleted to the left.
            let mut j = i + 1;

            // This is possible if the entry to be deleted is actually the last element
            // of the inner vector. In this case, we actually don't have any elements (to the left) to shift
            // so all we do is directly overwrite the value at index i to be a `MapEntry::VacantEntry`
            if j >= self.inner.len() {
                self.inner[i] = MapEntry::VacantEntry;
                self.num_items -= 1;
                return Ok(());
            }

            loop {
                let cur = self.inner.get(j).unwrap();

                // We overflow the bucket if we find an entry with psl == 0.
                // We can also stop if we see a vacant entry because there can't be any valid
                // occupied entries after a vacant entry (unless we overflow to the next bucket)
                if let MapEntry::Occupied(entry) = cur {
                    if entry.psl == 0 {
                        break;
                    }
                } else {
                    break;
                }

                j += 1;
            }

            // unsafe because UB if we go out of bounds, any of the pointers are invalid or we mess up the vec pointer while modifying
            // we should guarantee that indices i and j are within bounds
            unsafe {
                // Replace the entry to be deleted by shifting j - i - 1 elements to the left, overwriting
                // the entry to be deleted in the process
                let entry_ptr = self.inner.as_mut_ptr().add(i);
                ptr::copy(entry_ptr.offset(1), entry_ptr, j - i - 1);
                // We have to ensure that we add back in a `VacantEntry` after shifting all the elements of the bucket
                // thereby taking the place of the deleted entry in order to not mess up the vec's structure.
                ptr::write(entry_ptr.add(j - i - 1), MapEntry::VacantEntry);
            }

            self.num_items -= 1;
            return Ok(());
        } else {
            return Err("Entry not found");
        }
    }

    fn insert_entry(&mut self, mut entry: Entry<K, V>) {
        let slot = entry.hash % self.inner.len();
        let mut i = slot;

        loop {
            let cur = self.inner.get_mut(i);
            // We've probably reached the end of the backing vector after probing and not finding an empty spot. We'll just append the new entry at this point.
            // I'm not sure if this can ever happen but I'll just put it in here as a failsafe
            if let None = cur {
                self.inner.push(MapEntry::Occupied(entry));
                break;
            }

            let cur = cur.unwrap();
            if let MapEntry::Occupied(occupied_entry) = cur {
                if occupied_entry.key == entry.key {
                    // Update value
                    let _ = std::mem::replace(occupied_entry, entry);
                    // Return to prevent updating num items.
                    return;
                }

                if entry.psl > occupied_entry.psl {
                    std::mem::swap(&mut entry, occupied_entry);
                    continue;
                }

                i += 1;
            } else {
                // Insert entry into the vacancy.
                let _ = std::mem::replace(cur, MapEntry::Occupied(entry));
                break;
            }

            entry.psl += 1;
            self.max_psl = max(self.max_psl, entry.psl);
        }

        self.num_items += 1;
    }

    /// Gets the appropriate value given a valid key. Returns `None` if the key value mapping does not exist.
    ///
    /// From http://cglab.ca/~morin/publications/hashing/robinhood-siamjc.pdf:
    /// We hash ~ alpha*n elements into a table of size n where each probe is independent and uniformly distributed
    /// over the table, and alpha < 1 is a constant. Let M be the maximum search time for any of the elements in the table.
    /// We show that with probability tending to one, M is in [log2log n + a, log2log n + b]
    /// for some constants a and b depending upon alpha only. This is an exponential improvement
    /// over the maximum search time in case of the standard FCFS collision strategy.
    ///
    /// tl;dr - In general, even in the worst case, we can effectively consider lookup to be O(1) time.
    pub fn get(&self, key: &K) -> Option<&V> {
        if let Some(entry) = self.get_entry(key) {
            return Some(&entry.value);
        } else {
            return None;
        }
    }

    /// There are some additional (minor) optimizations in place here. Namely:
    /// We return nothing if we encounter an entry with a psl less than the number of steps we've walked.
    /// We return nothing if the number of steps we've walked exceeds the maximum psl value ever recorded.
    fn get_entry(&self, key: &K) -> Option<&Entry<K, V>> {
        let hash = self.hash_key(key);
        let slot = hash % self.inner.len();
        let mut d = slot;

        while d < self.inner.len() {
            let cur = self.inner.get(d).unwrap();
            if let MapEntry::Occupied(entry) = cur {
                if entry.key == *key {
                    return Some(entry);
                }

                // If we walked d steps and we encounter an entry that is some distance less than d from its home, we can stop.
                // OR: Our probing has reached to a point where it is impossible to find an entry this far out from home so we
                // can confidently stop in this case as well.
                if entry.psl < d || d > self.max_psl {
                    return None;
                }

                if d > self.max_psl {
                    return None;
                }
            } else {
                return None;
            }

            d += 1;
        }

        return None;
    }

    /// Clears all entries but preserves the allocated memory for use later.
    pub fn clear(&mut self) {
        let old_capacity = self.inner.len();
        self.inner.clear();

        let mut i = 0;
        while i < old_capacity {
            self.inner.push(MapEntry::VacantEntry);
            i += 1;
        }

        self.num_items = 0;
    }

    /// Checks to see if a value is associated with the given key.
    pub fn contains_key(&self, key: &K) -> bool {
        let entry = self.get_entry(key);
        if let Some(_) = entry {
            return true;
        } else {
            return false;
        }
    }

    /// Gets the length / number of entries of the hashmap.
    pub fn len(&self) -> usize {
        self.num_items
    }

    /// Gets the capacity of the hashmap.
    pub fn capacity(&self) -> usize {
        self.inner.len()
    }

    /// Allocates a new map of a different size and then moves the contents of the previous map into it.
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
            // Transfer ownership
            new_map.insert_entry(entry);
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
        let hashmap: RHMap<&str, i32, FxBuildHasher> = RHMap::with_capacity(initial_capacity);

        assert_eq!(hashmap.capacity(), initial_capacity);
    }

    #[test]
    fn it_inserts_values_without_initial_capacity() {
        let mut hashmap = RHMap::new();

        for x in 0..100 {
            hashmap.insert(x, x + 1);
        }

        assert_eq!(hashmap.len(), 100);

        for x in 100..0 {
            let val = hashmap.get(&x).unwrap();
            assert_eq!(*val, x + 1);
        }
    }

    #[test]
    fn it_inserts_values_with_initial_capacity() {
        let mut book_reviews = RHMap::with_capacity(10);
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

    #[test]
    fn it_checks_if_entry_exists() {
        let mut hashmap = RHMap::new();
        hashmap.insert(1, 2);

        assert_eq!(hashmap.contains_key(&1), true);
        assert_eq!(hashmap.contains_key(&2), false);
    }

    #[test]
    fn it_clears_all_entries() {
        let mut hashmap = RHMap::with_capacity(70);
        hashmap.insert(42, 0);
        hashmap.insert(42, 1);
        hashmap.clear();

        assert_eq!(hashmap.capacity(), 70);
        assert_eq!(hashmap.len(), 0);
        assert_eq!(hashmap.contains_key(&42), false);
    }

    #[test]
    #[allow(unused_must_use)]
    fn it_removes_entries() {
        let mut hashmap = RHMap::new();
        hashmap.insert(1, 2);
        hashmap.insert(4, 2);
        hashmap.insert(7, 2);
        hashmap.insert(3, 2);
        hashmap.insert(9, 2);

        hashmap.remove(&4);
        hashmap.remove(&1);
        hashmap.remove(&7);
        hashmap.remove(&3);

        assert!(!hashmap.contains_key(&4));
        assert!(!hashmap.contains_key(&1));
        assert!(!hashmap.contains_key(&7));
        assert!(!hashmap.contains_key(&3));
        assert_eq!(hashmap.len(), 4)
    }

    #[test]
    #[allow(unused_must_use)]
    fn it_removes_edge_case_entry() {
        // An edge case entry
        let mut hashmap = RHMap::with_capacity(1);
        hashmap.insert(1, 2);
        hashmap.remove(&1);
        assert!(!hashmap.contains_key(&1));
        assert_eq!(hashmap.len(), 0);
        assert_eq!(hashmap.capacity(), 1);
    }
}
