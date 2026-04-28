use std::ops;

use adpl_arena::Interned;
use adpl_hir as hir;

pub use adpl_arena::Index;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
}

#[derive(Clone, Copy, Debug)]
pub struct TryFromHirError;

impl TryFrom<hir::BinaryKind> for BinaryOp {
    type Error = TryFromHirError;

    fn try_from(value: hir::BinaryKind) -> Result<Self, Self::Error> {
        match value {
            hir::BinaryKind::Add => Ok(BinaryOp::Add),
            hir::BinaryKind::Sub => Ok(BinaryOp::Sub),
            hir::BinaryKind::Mul => Ok(BinaryOp::Mul),
            hir::BinaryKind::Div => Ok(BinaryOp::Div),
            hir::BinaryKind::Pow => Ok(BinaryOp::Pow),
            _ => Err(TryFromHirError),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Expression {
    Param(u16),
    Const(u64),
    Neg(Index<Expression>),
    Binary(BinaryOp, Index<Expression>, Index<Expression>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Predicate {
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
}

impl TryFrom<hir::BinaryKind> for Predicate {
    type Error = TryFromHirError;

    fn try_from(value: hir::BinaryKind) -> Result<Self, Self::Error> {
        match value {
            hir::BinaryKind::Eq => Ok(Predicate::Eq),
            hir::BinaryKind::Ne => Ok(Predicate::Ne),
            hir::BinaryKind::Gt => Ok(Predicate::Gt),
            hir::BinaryKind::Ge => Ok(Predicate::Ge),
            hir::BinaryKind::Lt => Ok(Predicate::Lt),
            hir::BinaryKind::Le => Ok(Predicate::Le),
            _ => Err(TryFromHirError),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Proposition {
    Not(Index<Proposition>),
    And(Index<Proposition>, Index<Proposition>),
    Or(Index<Proposition>, Index<Proposition>),
    Relation(Predicate, Index<Expression>, Index<Expression>),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Type {
    Real,
    Integer,
    Bool,
    Int(Index<Expression>),
    UInt(Index<Expression>),
    Ieee {
        exponent: Index<Expression>,
        fraction: Index<Expression>,
    },
    Record {
        name: Index<hir::Record>,
        args: Box<[Index<Expression>]>,
    },
}

pub struct PrimitiveTypes {
    pub real: Index<Type>,
    pub integer: Index<Type>,
    pub bool: Index<Type>,
}

pub struct TypeArenas {
    pub exprs: Interned<Expression>,
    pub props: Interned<Proposition>,
    pub types: Interned<Type>,
    pub prims: PrimitiveTypes,
}

impl TypeArenas {
    pub fn new() -> Self {
        let mut types = Interned::new();

        let prims = PrimitiveTypes {
            real: types.intern(Type::Real),
            integer: types.intern(Type::Integer),
            bool: types.intern(Type::Bool),
        };

        TypeArenas {
            exprs: Interned::new(),
            props: Interned::new(),
            types,
            prims,
        }
    }

    #[allow(private_bounds)]
    #[inline]
    pub fn intern<T>(&mut self, value: T) -> Index<T>
    where
        TypeArenas: Intern<T>,
    {
        Intern::intern(self, value)
    }
}

impl Default for TypeArenas {
    #[inline]
    fn default() -> Self {
        TypeArenas::new()
    }
}

impl<T> ops::Index<Index<T>> for TypeArenas
where
    TypeArenas: Intern<T>,
{
    type Output = T;

    #[inline]
    fn index(&self, index: Index<T>) -> &T {
        self.interner().index(index)
    }
}

trait Intern<T> {
    fn interner(&self) -> &Interned<T>;

    fn intern(&mut self, value: T) -> Index<T>;
}

macro_rules! intern_impl {
    ($field:ident, $ty:ty) => {
        impl Intern<$ty> for TypeArenas {
            #[inline]
            fn interner(&self) -> &Interned<$ty> {
                &self.$field
            }

            #[inline]
            fn intern(&mut self, value: $ty) -> Index<$ty> {
                self.$field.intern(value)
            }
        }
    };
}

intern_impl!(exprs, Expression);
intern_impl!(props, Proposition);
intern_impl!(types, Type);
