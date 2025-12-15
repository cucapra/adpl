use std::ops;

use adpl_arena::{Arena, Index, IndexArena, List};

use crate::hir;

#[derive(Default)]
pub struct Context {
    pub records: Arena<hir::Record>,
    pub fields: Arena<hir::Field>,
    pub defs: Arena<hir::Definition>,
    pub params: Arena<hir::Parameter>,
    pub stmts: Arena<hir::Statement>,
    pub exprs: Arena<hir::Expression>,
    pub types: Arena<hir::Type>,
    pub locals: Arena<hir::Local>,
    pub lists: IndexArena,
}

impl Context {
    #[inline]
    pub fn new() -> Self {
        Context::default()
    }
}

impl<T> ops::Index<Index<T>> for Context
where
    Context: Store<T>,
{
    type Output = T;

    #[inline]
    fn index(&self, index: Index<T>) -> &T {
        self.arena().index(index)
    }
}

impl<T> ops::IndexMut<Index<T>> for Context
where
    Context: Store<T>,
{
    #[inline]
    fn index_mut(&mut self, index: Index<T>) -> &mut T {
        self.mut_arena().index_mut(index)
    }
}

impl<T> ops::Index<List<T>> for Context {
    type Output = [Index<T>];

    #[inline]
    fn index(&self, index: List<T>) -> &[Index<T>] {
        self.lists.index(index)
    }
}

impl<T> ops::IndexMut<List<T>> for Context {
    #[inline]
    fn index_mut(&mut self, index: List<T>) -> &mut [Index<T>] {
        self.lists.index_mut(index)
    }
}

trait Store<T> {
    fn arena(&self) -> &Arena<T>;

    fn mut_arena(&mut self) -> &mut Arena<T>;
}

macro_rules! store_impl {
    ($field:ident, $ty:ty) => {
        impl Store<$ty> for Context {
            #[inline]
            fn arena(&self) -> &Arena<$ty> {
                &self.$field
            }

            #[inline]
            fn mut_arena(&mut self) -> &mut Arena<$ty> {
                &mut self.$field
            }
        }
    };
}

store_impl!(records, hir::Record);
store_impl!(fields, hir::Field);
store_impl!(defs, hir::Definition);
store_impl!(params, hir::Parameter);
store_impl!(stmts, hir::Statement);
store_impl!(exprs, hir::Expression);
store_impl!(types, hir::Type);
store_impl!(locals, hir::Local);
