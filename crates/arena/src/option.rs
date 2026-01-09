use std::cmp::Ordering;
use std::marker::PhantomData;
use std::num::NonZero;
use std::{fmt, ops};

use crate::index::{Index, IndexInner};

pub struct NonMaxIndex<T> {
    inner: NonZero<IndexInner>,
    phantom: PhantomData<fn() -> T>,
}

impl<T> NonMaxIndex<T> {
    #[inline]
    pub fn new(index: Index<T>) -> Option<Self> {
        Some(NonMaxIndex {
            inner: NonZero::new(!index.inner())?,
            phantom: PhantomData,
        })
    }

    #[inline]
    pub const fn zero() -> Self {
        NonMaxIndex {
            inner: unsafe { NonZero::new_unchecked(!0) },
            phantom: PhantomData,
        }
    }

    #[inline]
    pub fn get(self) -> Index<T> {
        Index::new_unchecked(!self.inner.get())
    }
}

impl<T> Clone for NonMaxIndex<T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for NonMaxIndex<T> {}

impl<T> PartialEq for NonMaxIndex<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.inner.eq(&other.inner)
    }
}

impl<T> Eq for NonMaxIndex<T> {}

impl<T> PartialOrd for NonMaxIndex<T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for NonMaxIndex<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.inner.cmp(&other.inner).reverse()
    }
}

impl<T> ops::Sub for NonMaxIndex<T> {
    type Output = IndexInner;

    #[inline]
    fn sub(self, rhs: Self) -> IndexInner {
        rhs.inner.get() - self.inner.get() // x - y = !y - !x
    }
}

impl<T> fmt::Debug for NonMaxIndex<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("NonMaxIndex")
            .field(&self.get().inner())
            .finish()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TryFromIndexError;

impl<T> TryFrom<Index<T>> for NonMaxIndex<T> {
    type Error = TryFromIndexError;

    #[inline]
    fn try_from(value: Index<T>) -> Result<Self, Self::Error> {
        NonMaxIndex::new(value).ok_or(TryFromIndexError)
    }
}
