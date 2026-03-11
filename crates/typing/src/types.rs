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
            types,
            prims,
        }
    }
}

impl Default for TypeArenas {
    #[inline]
    fn default() -> Self {
        TypeArenas::new()
    }
}

impl ops::Index<Index<Expression>> for TypeArenas {
    type Output = Expression;

    #[inline]
    fn index(&self, index: Index<Expression>) -> &Expression {
        self.exprs.index(index)
    }
}

impl ops::Index<Index<Type>> for TypeArenas {
    type Output = Type;

    #[inline]
    fn index(&self, index: Index<Type>) -> &Type {
        self.types.index(index)
    }
}

pub(crate) trait Intern<T>: ops::Index<Index<T>, Output = T> {
    fn intern(&mut self, value: T) -> Index<T>;
}

impl Intern<Expression> for TypeArenas {
    #[inline]
    fn intern(&mut self, value: Expression) -> Index<Expression> {
        self.exprs.intern(value)
    }
}

impl Intern<Type> for TypeArenas {
    #[inline]
    fn intern(&mut self, value: Type) -> Index<Type> {
        self.types.intern(value)
    }
}
