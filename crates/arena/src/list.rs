use std::fmt;
use std::marker::PhantomData;
use std::ops::{self, Range};

use crate::index::{Index, IndexInner, IndexSlice, InnerSlice};
use crate::option::NonMaxIndex;

pub struct List<T> {
    start: NonMaxIndex<IndexInner>,
    end: NonMaxIndex<IndexInner>,
    phantom: PhantomData<fn() -> T>,
}

impl<T> List<T> {
    /// Creates a new, empty list.
    #[inline]
    pub const fn new() -> Self {
        List {
            start: NonMaxIndex::zero(),
            end: NonMaxIndex::zero(),
            phantom: PhantomData,
        }
    }

    fn from_range(start: usize, end: usize) -> Self {
        List {
            start: NonMaxIndex::new(Index::from_usize(start)).unwrap(),
            end: NonMaxIndex::new(Index::from_usize(end)).unwrap(),
            phantom: PhantomData,
        }
    }

    #[inline]
    fn index(self) -> Range<usize> {
        self.start.get().index()..self.end.get().index()
    }

    #[inline]
    pub fn len(self) -> usize {
        (self.end - self.start) as usize
    }
}

impl<T> Clone for List<T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for List<T> {}

impl<T> fmt::Debug for List<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("List")
            .field("start", &self.start.get().inner())
            .field("end", &self.end.get().inner())
            .finish()
    }
}

#[derive(Default)]
pub struct IndexArena {
    data: Vec<IndexInner>,
}

impl IndexArena {
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }

    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        IndexArena {
            data: Vec::with_capacity(capacity),
        }
    }

    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.data.reserve(additional);
    }

    #[inline]
    pub fn reserve_exact(&mut self, additional: usize) {
        self.data.reserve_exact(additional);
    }

    pub fn extend_from_slice<T>(&mut self, slice: &[Index<T>]) -> List<T> {
        let start = self.data.len();
        self.data.extend_from_slice(slice.as_inner());

        List::from_range(start, self.data.len())
    }

    pub fn extend<I, T>(&mut self, iter: I) -> List<T>
    where
        I: IntoIterator<Item = Index<T>>,
    {
        let start = self.data.len();
        self.data.extend(iter.into_iter().map(Index::inner));

        List::from_range(start, self.data.len())
    }
}

impl<T> ops::Index<List<T>> for IndexArena {
    type Output = [Index<T>];

    #[inline]
    fn index(&self, index: List<T>) -> &[Index<T>] {
        self.data[index.index()].as_index()
    }
}

impl<T> ops::IndexMut<List<T>> for IndexArena {
    #[inline]
    fn index_mut(&mut self, index: List<T>) -> &mut [Index<T>] {
        self.data[index.index()].as_mut_index()
    }
}
