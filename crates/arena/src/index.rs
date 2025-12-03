use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::iter::FusedIterator;
use std::marker::PhantomData;
use std::ops::Range;

pub(crate) type IndexInner = u32;

pub struct Index<T> {
    index: IndexInner,
    phantom: PhantomData<fn() -> T>,
}

impl<T> Index<T> {
    #[inline]
    pub fn new(index: u32) -> Self {
        Index {
            index,
            phantom: PhantomData,
        }
    }

    #[inline]
    pub(crate) fn inner(self) -> IndexInner {
        self.index
    }

    #[inline]
    pub fn from_usize(index: usize) -> Self {
        Index::new(IndexInner::try_from(index).unwrap())
    }

    #[inline]
    pub fn index(self) -> usize {
        self.index as usize
    }
}

impl<T> Clone for Index<T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Index<T> {}

impl<T> PartialEq for Index<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.index.eq(&other.index)
    }
}

impl<T> Eq for Index<T> {}

impl<T> Hash for Index<T> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.index.hash(state)
    }
}

impl<T> PartialOrd for Index<T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for Index<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.index.cmp(&other.index)
    }
}

impl<T> fmt::Debug for Index<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("Index").field(&self.index).finish()
    }
}

pub struct IndexRange<T> {
    pub start: Index<T>,
    pub end: Index<T>,
}

impl<T> Clone for IndexRange<T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for IndexRange<T> {}

impl<T> fmt::Debug for IndexRange<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("IndexRange")
            .field("start", &self.start.index)
            .field("end", &self.end.index)
            .finish()
    }
}

impl<T> From<IndexRange<T>> for Range<Index<T>> {
    #[inline]
    fn from(value: IndexRange<T>) -> Self {
        Range {
            start: value.start,
            end: value.end,
        }
    }
}

impl<T> IntoIterator for IndexRange<T> {
    type Item = Index<T>;
    type IntoIter = IndexRangeIterator<T>;

    #[inline]
    fn into_iter(self) -> IndexRangeIterator<T> {
        IndexRangeIterator::new(self.start, self.end)
    }
}

pub struct IndexRangeIterator<T> {
    range: Range<IndexInner>,
    phantom: PhantomData<fn() -> T>,
}

impl<T> IndexRangeIterator<T> {
    #[inline]
    pub(crate) fn new(start: Index<T>, end: Index<T>) -> Self {
        IndexRangeIterator {
            range: start.index..end.index,
            phantom: PhantomData,
        }
    }
}

impl<T> Iterator for IndexRangeIterator<T> {
    type Item = Index<T>;

    #[inline]
    fn next(&mut self) -> Option<Index<T>> {
        Some(Index::new(self.range.next()?))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.range.size_hint()
    }
}

impl<T> DoubleEndedIterator for IndexRangeIterator<T> {
    #[inline]
    fn next_back(&mut self) -> Option<Index<T>> {
        Some(Index::new(self.range.next_back()?))
    }
}

impl<T> ExactSizeIterator for IndexRangeIterator<T> {
    #[inline]
    fn len(&self) -> usize {
        self.range.len()
    }
}

impl<T> FusedIterator for IndexRangeIterator<T> where
    Range<IndexInner>: FusedIterator
{
}
