use std::collections::HashMap;
use std::hash::Hash;
use std::ops;

use crate::{Arena, Index};

pub struct Interned<T> {
    arena: Arena<T>,
    index: HashMap<T, Index<T>>,
}

impl<T> Interned<T> {
    pub fn new() -> Self {
        Interned {
            arena: Arena::new(),
            index: HashMap::new(),
        }
    }
}

impl<T> Interned<T>
where
    T: Clone + Eq + Hash,
{
    pub fn intern(&mut self, value: T) -> Index<T> {
        *self
            .index
            .entry(value)
            .or_insert_with_key(|value| self.arena.push(value.clone()))
    }
}

impl<T> Default for Interned<T> {
    #[inline]
    fn default() -> Self {
        Interned::new()
    }
}

impl<T> ops::Index<Index<T>> for Interned<T> {
    type Output = T;

    #[inline]
    fn index(&self, index: Index<T>) -> &T {
        &self.arena[index]
    }
}
