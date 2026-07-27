use adpl_arena::{Index, Interned};

use crate::types::{Expression, Proposition, Type, TypeArenas};

pub trait Folder<T> {
    fn fold_param(&self, index: usize) -> T;

    fn fold_generic(&self, index: usize) -> T;
}

pub struct Arguments<'a> {
    pub generics: &'a [Index<Expression>],
    pub args: &'a [Index<Expression>],
}

impl<'a> Arguments<'a> {
    pub fn new(
        generics: &'a [Index<Expression>],
        args: &'a [Index<Expression>],
    ) -> Self {
        Arguments { generics, args }
    }
}

impl Folder<Index<Expression>> for Arguments<'_> {
    fn fold_param(&self, index: usize) -> Index<Expression> {
        self.args[index]
    }

    fn fold_generic(&self, index: usize) -> Index<Expression> {
        self.generics[index]
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
            Type::Real | Type::Integer | Type::Bool => self,
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
            Expression::GenericParam(i) => folder.fold_generic(i.into()),
            Expression::Term(_) => panic!(),
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

impl Foldable<Index<Expression>> for Index<Proposition> {
    type Context = TypeArenas;

    fn fold_with<F>(self, ctx: &mut Self::Context, folder: &F) -> Self
    where
        F: Folder<Index<Expression>> + ?Sized,
    {
        match ctx[self] {
            Proposition::Not(prop) => {
                let prop = prop.fold_with(ctx, folder);

                ctx.intern(Proposition::Not(prop))
            }
            Proposition::And(lhs, rhs) => {
                let lhs = lhs.fold_with(ctx, folder);
                let rhs = rhs.fold_with(ctx, folder);

                ctx.intern(Proposition::And(lhs, rhs))
            }
            Proposition::Or(lhs, rhs) => {
                let lhs = lhs.fold_with(ctx, folder);
                let rhs = rhs.fold_with(ctx, folder);

                ctx.intern(Proposition::Or(lhs, rhs))
            }
            Proposition::Relation(p, lhs, rhs) => {
                let lhs = lhs.fold_with(&mut ctx.exprs, folder);
                let rhs = rhs.fold_with(&mut ctx.exprs, folder);

                ctx.intern(Proposition::Relation(p, lhs, rhs))
            }
        }
    }
}
