use adpl_hir as hir;

use crate::types::{Index, Type, TypeArenas};

pub trait Promote<Rhs = Self> {
    type Output;
    type Context;

    fn promote(self, rhs: Rhs, ctx: &Self::Context) -> Self::Output;
}

impl Promote for Index<Type> {
    type Output = Option<Index<Type>>;
    type Context = TypeArenas;

    fn promote(
        self,
        rhs: Index<Type>,
        ctx: &TypeArenas,
    ) -> Option<Index<Type>> {
        match &ctx[self] {
            Type::Real => match &ctx[rhs] {
                Type::Real | Type::Integer | Type::Int(_) | Type::UInt(_) => {
                    Some(self)
                }
                _ => None,
            },
            Type::Integer => match &ctx[rhs] {
                Type::Real => Some(rhs),
                Type::Integer | Type::Int(_) | Type::UInt(_) => Some(self),
                _ => None,
            },
            Type::Bool => match &ctx[rhs] {
                Type::Bool => Some(self),
                _ => None,
            },
            Type::Int(_) | Type::UInt(_) => match &ctx[rhs] {
                Type::Real | Type::Integer => Some(rhs),
                Type::Int(_) | Type::UInt(_) => Some(ctx.prims.integer),
                _ => None,
            },
            _ => None,
        }
    }
}

pub trait Overloaded<Args> {
    fn select_overload(
        self,
        args: Args,
        ctx: &TypeArenas,
    ) -> Option<Index<Type>>;
}

impl Overloaded<(Index<Type>,)> for hir::UnaryKind {
    fn select_overload(
        self,
        args: (Index<Type>,),
        ctx: &TypeArenas,
    ) -> Option<Index<Type>> {
        match self {
            hir::UnaryKind::Neg => match &ctx[args.0] {
                Type::Real | Type::Integer => Some(args.0),
                Type::Int(_) | Type::UInt(_) => Some(ctx.prims.integer),
                _ => None,
            },
            hir::UnaryKind::Not => match &ctx[args.0] {
                Type::Bool => Some(args.0),
                _ => None,
            },
        }
    }
}

impl Overloaded<(Index<Type>, Index<Type>)> for hir::BinaryKind {
    fn select_overload(
        self,
        args: (Index<Type>, Index<Type>),
        ctx: &TypeArenas,
    ) -> Option<Index<Type>> {
        let common = Promote::promote(args.0, args.1, ctx)?;

        match self {
            hir::BinaryKind::Add
            | hir::BinaryKind::Sub
            | hir::BinaryKind::Mul => match &ctx[common] {
                Type::Real | Type::Integer => Some(common),
                _ => None,
            },
            hir::BinaryKind::Div => match &ctx[common] {
                Type::Real | Type::Integer => Some(ctx.prims.real),
                _ => None,
            },
            hir::BinaryKind::Pow => match &ctx[common] {
                Type::Integer => Some(ctx.prims.real),
                _ => None,
            },
            hir::BinaryKind::Shl | hir::BinaryKind::Shr => None,
            hir::BinaryKind::And | hir::BinaryKind::Or => match &ctx[common] {
                Type::Bool => Some(common),
                _ => None,
            },
            hir::BinaryKind::Eq | hir::BinaryKind::Ne => match &ctx[common] {
                Type::Real | Type::Integer => Some(ctx.prims.bool),
                _ => None,
            },
            hir::BinaryKind::Gt
            | hir::BinaryKind::Ge
            | hir::BinaryKind::Lt
            | hir::BinaryKind::Le => match &ctx[common] {
                Type::Real | Type::Integer => Some(ctx.prims.bool),
                _ => None,
            },
        }
    }
}
