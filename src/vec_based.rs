use crate::Error;

#[derive(Clone, Copy, Debug)]
pub struct GenIndex {
    index: usize,
    generation: u32,
}

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
                None => {
                    return Err(
                        simple_error::SimpleError::new("Entry that should exist not found").into(),
                    )
                }
                Some(entry) => {
                    entry.key.generation += 1;
                    entry.value.replace(value);
                    Ok(entry.key.clone())
                }
            },
        }
    }

    pub fn deallocate(&mut self, key: &GenIndex) -> Result<Option<T>, Error> {
        match self.entries.get_mut(key.index) {
            None => return Err(simple_error::SimpleError::new("Not found").into()),
            Some(entry) => {
                if entry.key.generation != key.generation {
                    return Err(simple_error::SimpleError::new("Wrong generation").into());
                }

                let value = entry.value.take();
                self.free_indices.push(key.index);
                Ok(value)
            }
        }
    }

    pub fn get(&self, key: &GenIndex) -> Option<&T> {
        match self.entries.get(key.index) {
            None => return None,
            Some(entry) => {
                if entry.key.generation != key.generation {
                    return None;
                }

                (&entry.value).as_ref()
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn create_and_allocate() {
        let mut gen_alloc = GenIndexAllocator::new();

        let mut alloced_keys: Vec<_> = (0..10)
            .into_iter()
            .map(|value| gen_alloc.allocate(value).expect("Should allocate"))
            .collect();
        dbg!(&alloced_keys);

        for key in alloced_keys.iter() {
            assert!(gen_alloc.get(key).is_some());
        }

        let to_free = alloced_keys.split_off(5);

        for key in to_free.iter() {
            gen_alloc.deallocate(key);
        }

        let new_alloced_keys: Vec<_> = (0..10)
            .into_iter()
            .map(|value| gen_alloc.allocate(value).expect("Should allocate"))
            .collect();
        dbg!(&new_alloced_keys);
    }
}
