use std::iter::{self, FusedIterator};
use std::{ops, slice};

use crate::{Index, IndexRangeIterator};

pub struct Arena<T> {
    data: Vec<T>,
}

impl<T> Arena<T> {
    #[inline]
    pub fn new() -> Self {
        Arena { data: Vec::new() }
    }

    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Arena {
            data: Vec::with_capacity(capacity),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.data.reserve(additional);
    }

    #[inline]
    pub fn reserve_exact(&mut self, additional: usize) {
        self.data.reserve_exact(additional);
    }

    #[inline]
    pub fn next_index(&self) -> Index<T> {
        Index::from_usize(self.data.len()).unwrap()
    }

    #[inline]
    pub fn push(&mut self, value: T) -> Index<T> {
        let index = self.next_index();
        self.data.push(value);

        index
    }

    #[inline]
    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            iter: self.data.iter().enumerate(),
        }
    }

    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut {
            iter: self.data.iter_mut().enumerate(),
        }
    }

    #[inline]
    pub fn keys(&self) -> IndexRangeIterator<T> {
        IndexRangeIterator::new(
            Index::new_unchecked(0),
            Index::from_usize_unchecked(self.data.len()),
        )
    }

    #[inline]
    pub fn values(&self) -> slice::Iter<'_, T> {
        self.data.iter()
    }

    #[inline]
    pub fn values_mut(&mut self) -> slice::IterMut<'_, T> {
        self.data.iter_mut()
    }
}

impl<T> Default for Arena<T> {
    #[inline]
    fn default() -> Self {
        Arena::new()
    }
}

impl<T> ops::Index<Index<T>> for Arena<T> {
    type Output = T;

    #[inline]
    fn index(&self, index: Index<T>) -> &T {
        &self.data[index.index()]
    }
}

impl<T> ops::IndexMut<Index<T>> for Arena<T> {
    #[inline]
    fn index_mut(&mut self, index: Index<T>) -> &mut T {
        &mut self.data[index.index()]
    }
}

pub struct Iter<'a, T: 'a> {
    iter: iter::Enumerate<slice::Iter<'a, T>>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (Index<T>, &'a T);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|(i, v)| (Index::from_usize_unchecked(i), v))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<T> DoubleEndedIterator for Iter<'_, T> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter
            .next_back()
            .map(|(i, v)| (Index::from_usize_unchecked(i), v))
    }
}

impl<T> ExactSizeIterator for Iter<'_, T> {
    #[inline]
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<'a, T> FusedIterator for Iter<'a, T> where
    iter::Enumerate<slice::Iter<'a, T>>: FusedIterator
{
}

pub struct IterMut<'a, T: 'a> {
    iter: iter::Enumerate<slice::IterMut<'a, T>>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (Index<T>, &'a mut T);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|(i, v)| (Index::from_usize_unchecked(i), v))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<T> DoubleEndedIterator for IterMut<'_, T> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter
            .next_back()
            .map(|(i, v)| (Index::from_usize_unchecked(i), v))
    }
}

impl<T> ExactSizeIterator for IterMut<'_, T> {
    #[inline]
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<'a, T> FusedIterator for IterMut<'a, T> where
    iter::Enumerate<slice::IterMut<'a, T>>: FusedIterator
{
}
