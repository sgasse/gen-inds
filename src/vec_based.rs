use crate::Error;

#[derive(Clone, Copy, Debug)]
pub struct GenIndKey {
    index: usize,
    generation: u32,
}

struct GenEntry<T> {
    key: GenIndKey,
    data: Option<T>,
}

pub struct GenAllocator<T> {
    entries: Vec<GenEntry<T>>,
    free_indices: Vec<usize>,
}

impl<T> GenAllocator<T> {
    pub fn new() -> Self {
        Self::with_capacity(100)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
            free_indices: Vec::new(),
        }
    }

    pub fn allocate(&mut self, value: T) -> Result<GenIndKey, Error> {
        match self.free_indices.pop() {
            None => {
                let new_key = GenIndKey {
                    index: self.entries.len(),
                    generation: 0,
                };
                self.entries.push(GenEntry {
                    key: new_key,
                    data: Some(value),
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
                    entry.data.replace(value);
                    Ok(entry.key.clone())
                }
            },
        }
    }

    pub fn deallocate(&mut self, key: &GenIndKey) -> Result<Option<T>, Error> {
        match self.entries.get_mut(key.index) {
            None => return Err(simple_error::SimpleError::new("Not found").into()),
            Some(entry) => {
                if entry.key.generation != key.generation {
                    return Err(simple_error::SimpleError::new("Wrong generation").into());
                }

                let value = entry.data.take();
                self.free_indices.push(key.index);
                Ok(value)
            }
        }
    }

    pub fn get(&self, key: &GenIndKey) -> Option<&T> {
        match self.entries.get(key.index) {
            None => return None,
            Some(entry) => {
                if entry.key.generation != key.generation {
                    return None;
                }

                (&entry.data).as_ref()
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::GenAllocator;

    #[test]
    fn create_and_allocate() {
        let mut gen_alloc = GenAllocator::new();

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
