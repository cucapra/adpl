use adpl_arena::{Index, Interned};

use crate::types::{Expression, Type, TypeArenas};

pub trait Folder<T> {
    fn fold_param(&self, index: usize) -> T;
}

impl Folder<Index<Expression>> for [Index<Expression>] {
    fn fold_param(&self, index: usize) -> Index<Expression> {
        self[index]
    }
}

pub trait Foldable<T> {
    type Context;

    fn fold_with<F>(self, ctx: &mut Self::Context, folder: &F) -> Self
    where
        F: Folder<T> + ?Sized;
}

impl Foldable<Index<Expression>> for Index<Type> {
    type Context = TypeArenas;

    fn fold_with<F>(self, ctx: &mut Self::Context, folder: &F) -> Self
    where
        F: Folder<Index<Expression>> + ?Sized,
    {
        match ctx.types[self] {
            Type::Real | Type::UnsizedInteger | Type::Bool => self,
            Type::Int(width) => {
                let width = width.fold_with(&mut ctx.exprs, folder);

                ctx.types.intern(Type::Int(width))
            }
            Type::UInt(width) => {
                let width = width.fold_with(&mut ctx.exprs, folder);

                ctx.types.intern(Type::UInt(width))
            }
            Type::Ieee { exponent, fraction } => {
                let exponent = exponent.fold_with(&mut ctx.exprs, folder);
                let fraction = fraction.fold_with(&mut ctx.exprs, folder);

                ctx.types.intern(Type::Ieee { exponent, fraction })
            }
            Type::Record { name, ref args } => {
                let args = args
                    .iter()
                    .map(|arg| arg.fold_with(&mut ctx.exprs, folder))
                    .collect();

                ctx.types.intern(Type::Record { name, args })
            }
        }
    }
}

impl Foldable<Index<Expression>> for Index<Expression> {
    type Context = Interned<Expression>;

    fn fold_with<F>(self, ctx: &mut Self::Context, folder: &F) -> Self
    where
        F: Folder<Index<Expression>> + ?Sized,
    {
        match ctx[self] {
            Expression::Param(i) => folder.fold_param(i.into()),
            Expression::Const(_) => self,
            Expression::Neg(expr) => {
                let expr = expr.fold_with(ctx, folder);

                ctx.intern(Expression::Neg(expr))
            }
            Expression::Binary(op, lhs, rhs) => {
                let lhs = lhs.fold_with(ctx, folder);
                let rhs = rhs.fold_with(ctx, folder);

                ctx.intern(Expression::Binary(op, lhs, rhs))
            }
        }
    }
}
