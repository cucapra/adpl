use std::convert::Infallible;
use std::ops::ControlFlow;

use crate::types::{Expression, Index, Proposition, Type, TypeArenas};

pub trait VisitResult {
    type Residual;

    fn output() -> Self;

    fn from_residual(residual: Self::Residual) -> Self;

    fn branch(self) -> ControlFlow<Self::Residual>;
}

impl VisitResult for () {
    type Residual = Infallible;

    fn output() {}

    fn from_residual(_: Infallible) {}

    fn branch(self) -> ControlFlow<Infallible> {
        ControlFlow::Continue(())
    }
}

pub trait Visitor<'ctx, Context> {
    type Result: VisitResult;

    fn ctx(&self) -> &'ctx Context;

    fn visit_param(&mut self, _index: usize) -> Self::Result {
        VisitResult::output()
    }

    fn visit_generic(&mut self, _index: usize) -> Self::Result {
        VisitResult::output()
    }
}

pub trait Visitable<Context> {
    fn visit_with<'ctx, V>(self, visitor: &mut V) -> V::Result
    where
        V: Visitor<'ctx, Context>;
}

macro_rules! try_visit {
    ($e:expr, $visitor:expr) => {
        match $e.visit_with($visitor).branch() {
            ControlFlow::Continue(()) => {}
            ControlFlow::Break(residual) => {
                return VisitResult::from_residual(residual);
            }
        }
    };
}

impl<Context, T: Visitable<Context> + Copy> Visitable<Context> for &[T] {
    fn visit_with<'ctx, V>(self, visitor: &mut V) -> V::Result
    where
        V: Visitor<'ctx, Context>,
    {
        for e in self {
            try_visit!(e, visitor);
        }

        VisitResult::output()
    }
}

impl Visitable<TypeArenas> for Index<Type> {
    fn visit_with<'ctx, V>(self, visitor: &mut V) -> V::Result
    where
        V: Visitor<'ctx, TypeArenas>,
    {
        match visitor.ctx()[self] {
            Type::Real | Type::Integer | Type::Bool => VisitResult::output(),
            Type::Int(width) => width.visit_with(visitor),
            Type::UInt(width) => width.visit_with(visitor),
            Type::Ieee { exponent, fraction } => {
                try_visit!(exponent, visitor);

                fraction.visit_with(visitor)
            }
            Type::Record { name: _, ref args } => args.visit_with(visitor),
        }
    }
}

impl Visitable<TypeArenas> for Index<Expression> {
    fn visit_with<'ctx, V>(self, visitor: &mut V) -> V::Result
    where
        V: Visitor<'ctx, TypeArenas>,
    {
        match visitor.ctx()[self] {
            Expression::Param(i) => visitor.visit_param(i.into()),
            Expression::GenericParam(i) => visitor.visit_generic(i.into()),
            Expression::Term(_) => VisitResult::output(),
            Expression::Const(_) => VisitResult::output(),
            Expression::Neg(expr) => expr.visit_with(visitor),
            Expression::Binary(_, lhs, rhs) => {
                try_visit!(lhs, visitor);

                rhs.visit_with(visitor)
            }
        }
    }
}

impl Visitable<TypeArenas> for Index<Proposition> {
    fn visit_with<'ctx, V>(self, visitor: &mut V) -> V::Result
    where
        V: Visitor<'ctx, TypeArenas>,
    {
        match visitor.ctx()[self] {
            Proposition::Not(prop) => prop.visit_with(visitor),
            Proposition::And(lhs, rhs) | Proposition::Or(lhs, rhs) => {
                try_visit!(lhs, visitor);

                rhs.visit_with(visitor)
            }
            Proposition::Relation(_, lhs, rhs) => {
                try_visit!(lhs, visitor);

                rhs.visit_with(visitor)
            }
        }
    }
}
