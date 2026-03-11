use adpl_hir as hir;

use crate::types::{Index, Type, TypeArenas};

pub trait Lub<Rhs = Self> {
    type Output;
    type Context;

    fn lub(self, rhs: Rhs, ctx: &Self::Context) -> Self::Output;
}

impl Lub for Index<Type> {
    type Output = Option<Index<Type>>;
    type Context = TypeArenas;

    fn lub(self, rhs: Index<Type>, ctx: &TypeArenas) -> Option<Index<Type>> {
        if self == rhs {
            return Some(self);
        }

        match (&ctx[self], &ctx[rhs]) {
            (Type::Real, Type::Integer) => Some(self),
            (Type::Integer, Type::Real) => Some(rhs),
            _ => None,
        }
    }
}

pub trait Overloaded: Sized {
    fn select_overload(
        self,
        lub: Index<Type>,
        ctx: &TypeArenas,
    ) -> Option<Index<Type>>;

    fn select_binary_overload(
        self,
        lhs: Index<Type>,
        rhs: Index<Type>,
        ctx: &TypeArenas,
    ) -> Option<Index<Type>> {
        self.select_overload(lhs.lub(rhs, ctx)?, ctx)
    }
}

impl Overloaded for hir::UnaryKind {
    fn select_overload(
        self,
        lub: Index<Type>,
        ctx: &TypeArenas,
    ) -> Option<Index<Type>> {
        match self {
            hir::UnaryKind::Neg => match &ctx[lub] {
                Type::Real | Type::Integer => Some(lub),
                _ => None,
            },
            hir::UnaryKind::Not => match &ctx[lub] {
                Type::Bool => Some(lub),
                _ => None,
            },
        }
    }
}

impl Overloaded for hir::BinaryKind {
    fn select_overload(
        self,
        lub: Index<Type>,
        ctx: &TypeArenas,
    ) -> Option<Index<Type>> {
        match self {
            hir::BinaryKind::Add
            | hir::BinaryKind::Sub
            | hir::BinaryKind::Mul => match &ctx[lub] {
                Type::Real | Type::Integer => Some(lub),
                _ => None,
            },
            hir::BinaryKind::Div | hir::BinaryKind::Pow => match &ctx[lub] {
                Type::Real | Type::Integer => Some(ctx.prims.real),
                _ => None,
            },
            hir::BinaryKind::Shl | hir::BinaryKind::Shr => None,
            hir::BinaryKind::Eq | hir::BinaryKind::Ne => match &ctx[lub] {
                Type::Real | Type::Integer | Type::Bool => Some(ctx.prims.bool),
                _ => None,
            },
            hir::BinaryKind::Gt
            | hir::BinaryKind::Ge
            | hir::BinaryKind::Lt
            | hir::BinaryKind::Le => match &ctx[lub] {
                Type::Real | Type::Integer => Some(ctx.prims.bool),
                _ => None,
            },
        }
    }
}
