use std::marker::PhantomData;
use std::ops;

use crate::Index;

pub struct DenseMap<K, V> {
    data: Vec<V>,
    phantom: PhantomData<fn() -> K>,
}

impl<K, V> DenseMap<K, V> {
    #[inline]
    pub fn new() -> Self {
        DenseMap {
            data: Vec::new(),
            phantom: PhantomData,
        }
    }

    #[inline]
    pub fn insert_back(&mut self, index: Index<K>, value: V) {
        assert!(index.index() == self.data.len());

        self.data.push(value);
    }
}

impl<K, V: Clone> DenseMap<K, V> {
    #[inline]
    pub fn filled(len: usize, value: V) -> Self {
        DenseMap {
            data: vec![value; len],
            phantom: PhantomData,
        }
    }

    #[inline]
    pub fn filled_with_default(len: usize) -> Self
    where
        V: Default,
    {
        DenseMap::filled(len, Default::default())
    }

    #[inline]
    pub fn resize(&mut self, len: usize, value: V) {
        self.data.resize(len, value)
    }
}

impl<K, V> Default for DenseMap<K, V> {
    #[inline]
    fn default() -> Self {
        DenseMap::new()
    }
}

impl<K, V> FromIterator<V> for DenseMap<K, V> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = V>>(iter: I) -> Self {
        DenseMap {
            data: Vec::from_iter(iter),
            phantom: PhantomData,
        }
    }
}

impl<K, V> ops::Index<Index<K>> for DenseMap<K, V> {
    type Output = V;

    #[inline]
    fn index(&self, index: Index<K>) -> &V {
        &self.data[index.index()]
    }
}

impl<K, V> ops::IndexMut<Index<K>> for DenseMap<K, V> {
    #[inline]
    fn index_mut(&mut self, index: Index<K>) -> &mut V {
        &mut self.data[index.index()]
    }
}
