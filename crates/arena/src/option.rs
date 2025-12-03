use std::fmt;
use std::marker::PhantomData;
use std::num::NonZero;

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
    pub fn get(self) -> Index<T> {
        Index::new(!self.inner.get())
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

impl<T> fmt::Debug for NonMaxIndex<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("NonMaxIndex")
            .field(&self.get().inner())
            .finish()
    }
}
