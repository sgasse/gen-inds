use crate::Error;
use simple_error::bail;

#[derive(Clone, Copy, Debug)]
pub struct GenIndex {
    index: usize,
    generation: u32,
}

#[derive(Debug)]
struct GenIndexEntry<T> {
    key: GenIndex,
    value: Option<T>,
}

pub struct GenIndexAllocator<T> {
    entries: Vec<GenIndexEntry<T>>,
    free_indices: Vec<usize>,
}

impl<T> GenIndexAllocator<T> {
    pub fn new() -> Self {
        Self::with_capacity(100)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
            free_indices: Vec::new(),
        }
    }

    pub fn allocate(&mut self, value: T) -> Result<GenIndex, Error> {
        match self.free_indices.pop() {
            None => {
                let new_key = GenIndex {
                    index: self.entries.len(),
                    generation: 0,
                };
                self.entries.push(GenIndexEntry {
                    key: new_key,
                    value: Some(value),
                });
                Ok(new_key)
            }
            Some(free_idx) => match self.entries.get_mut(free_idx) {
                None => bail!(
                    "GenIndexAllocator::allocate: Could not find free index that should exist"
                ),
                Some(entry) => {
                    entry.key.generation += 1;
                    entry.value.replace(value);
                    Ok(entry.key)
                }
            },
        }
    }

    pub fn deallocate(&mut self, key: &GenIndex) -> Result<Option<T>, Error> {
        match self.entries.get_mut(key.index) {
            None => bail!("GenIndexAllocator::deallocate: Index not found"),
            Some(entry) => {
                if entry.key.generation != key.generation {
                    bail!("GenIndexAllocator::deallocate: Wrong generation");
                }

                let value = entry.value.take();
                self.free_indices.push(key.index);
                Ok(value)
            }
        }
    }

    pub fn get(&self, key: &GenIndex) -> Option<&T> {
        match self.entries.get(key.index) {
            None => None,
            Some(entry) => {
                if entry.key.generation != key.generation {
                    return None;
                }

                (entry.value).as_ref()
            }
        }
    }

    pub fn get_mut(&mut self, key: &GenIndex) -> Option<&mut T> {
        match self.entries.get_mut(key.index) {
            None => None,
            Some(entry) => {
                if entry.key.generation != key.generation {
                    return None;
                }

                (entry.value).as_mut()
            }
        }
    }

    pub fn set(&mut self, key: &GenIndex, value: T) -> Result<T, Error> {
        match self.entries.get_mut(key.index) {
            None => bail!("GenIndexAllocator::set: Entry for key not found"),
            Some(entry) => {
                if entry.key.generation != key.generation {
                    bail!("GenIndexAllocator::set: Entry exists but generation does not match");
                }

                entry
                    .value
                    .replace(value)
                    .ok_or_else(|| {
                        simple_error::SimpleError::new(
                            "GenIndexAllocator::set: Entry to overwrite is empty but should not be",
                        )
                    })
                    .map_err(|e| e.into())
            }
        }
    }
}

impl<T> Default for GenIndexAllocator<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_create_with_capacity() -> Result<(), Error> {
        let capacity = 200;
        let gen_alloc = GenIndexAllocator::<i32>::with_capacity(capacity);
        assert_eq!(gen_alloc.entries.capacity(), capacity);
        Ok(())
    }

    #[test]
    fn test_allocate_and_get() -> Result<(), Error> {
        let mut gen_alloc = GenIndexAllocator::with_capacity(10);

        // Create value and check it
        let value1 = 1i32;
        let key1 = gen_alloc.allocate(value1)?;
        assert_eq!(gen_alloc.entries.len(), 1);
        assert_eq!(gen_alloc.get(&key1), Some(&value1));

        Ok(())
    }

    #[test]
    fn test_allocate_and_set() -> Result<(), Error> {
        let mut gen_alloc = GenIndexAllocator::with_capacity(10);

        // Create value and check it
        let value1 = 1i32;
        let key1 = gen_alloc.allocate(value1)?;
        assert_eq!(gen_alloc.entries.len(), 1);
        assert_eq!(gen_alloc.get(&key1), Some(&value1));

        // Create value and check it
        let value2 = 2i32;
        let key2 = gen_alloc.allocate(value2)?;
        assert_eq!(gen_alloc.entries.len(), 2);
        assert_eq!(gen_alloc.get(&key2), Some(&value2));

        // Set first key to different value - the second value should be unchanged
        let new_value1 = 99i32;
        gen_alloc.set(&key1, new_value1)?;
        assert_eq!(gen_alloc.entries.len(), 2);
        assert_eq!(gen_alloc.get(&key1), Some(&new_value1));
        assert_eq!(gen_alloc.get(&key2), Some(&value2));

        Ok(())
    }

    #[test]
    fn test_reuse_free_indices() -> Result<(), Error> {
        let capacity = 5;
        let mut gen_alloc = GenIndexAllocator::with_capacity(capacity);
        assert_eq!(gen_alloc.entries.len(), 0);
        assert_eq!(gen_alloc.entries.capacity(), capacity);

        let mut alloced_keys: Vec<_> = (0..capacity)
            .into_iter()
            .map(|value| gen_alloc.allocate(value).expect("Should allocate"))
            .collect();

        // We created the values in order - check that each key points to the right value
        for (value, key) in alloced_keys.iter().enumerate() {
            assert_eq!(gen_alloc.get(key), Some(&value));
        }

        // Split the keys and free some of them
        let num_keys_to_free = 2;
        let to_free = alloced_keys.split_off(capacity - num_keys_to_free);

        for key in to_free.iter() {
            gen_alloc.deallocate(key)?;
        }

        assert_eq!(
            gen_alloc.entries.len(),
            capacity,
            "We do not remove entries so the length should be unchanged"
        );
        assert_eq!(
            gen_alloc.entries.capacity(),
            capacity,
            "We do not exceed capacity so it should be unchanged"
        );

        // Reuse indices, the capacity should be unchanged but old keys should get invalid
        let reused_entries_keys: Vec<_> = (0..num_keys_to_free)
            .into_iter()
            .map(|value| gen_alloc.allocate(value).expect("Should allocate"))
            .collect();

        assert_eq!(
            gen_alloc.entries.len(),
            capacity,
            "We do not remove entries so the length should be unchanged"
        );
        assert_eq!(
            gen_alloc.entries.capacity(),
            capacity,
            "We do not exceed capacity so it should be unchanged"
        );

        for key in to_free.iter() {
            assert_eq!(
                gen_alloc.get(key),
                None,
                "This key should be invalid because the generation does not match"
            );
        }

        for (value, key) in reused_entries_keys.iter().enumerate() {
            assert_eq!(gen_alloc.get(key), Some(&value));
        }

        Ok(())
    }
}
