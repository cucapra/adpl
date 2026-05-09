use std::ops::ControlFlow;

use z3::{self, ast};

use crate::types::{self as ty, Index, Type, TypeArenas};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LBool {
    False,
    True,
    Unknown,
}

impl LBool {
    pub fn and_then<F: FnOnce() -> LBool>(self, f: F) -> LBool {
        match self {
            LBool::False => LBool::False,
            _ => self & f(),
        }
    }
}

impl From<bool> for LBool {
    #[inline]
    fn from(value: bool) -> Self {
        match value {
            false => LBool::False,
            true => LBool::True,
        }
    }
}

impl std::ops::BitAnd for LBool {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (LBool::False, _) | (_, LBool::False) => LBool::False,
            (LBool::Unknown, _) | (_, LBool::Unknown) => LBool::Unknown,
            (LBool::True, LBool::True) => LBool::True,
        }
    }
}

trait LiftedAll: Iterator + Sized {
    fn lifted_all<F>(&mut self, mut f: F) -> LBool
    where
        F: FnMut(Self::Item) -> LBool,
    {
        let result = self.try_fold(LBool::True, |acc, x| match f(x) {
            LBool::False => ControlFlow::Break(LBool::False),
            LBool::Unknown => ControlFlow::Continue(LBool::Unknown),
            LBool::True => ControlFlow::Continue(acc),
        });

        match result {
            ControlFlow::Continue(b) => b,
            ControlFlow::Break(b) => b,
        }
    }
}

impl<I: Iterator> LiftedAll for I {}

pub trait Equivalent<Rhs = Self> {
    type Context;

    fn equivalent(self, rhs: Rhs, ctx: &Self::Context) -> LBool;
}

impl Equivalent for Index<Type> {
    type Context = TypeArenas;

    fn equivalent(self, rhs: Self, ctx: &Self::Context) -> LBool {
        if self == rhs {
            return LBool::True;
        }

        match (&ctx[self], &ctx[rhs]) {
            (Type::Real, Type::Real) => LBool::True,
            (Type::Integer, Type::Integer) => LBool::True,
            (Type::Bool, Type::Bool) => LBool::True,
            (Type::Int(lhs), Type::Int(rhs)) => lhs.equivalent(*rhs, ctx),
            (Type::UInt(lhs), Type::UInt(rhs)) => lhs.equivalent(*rhs, ctx),
            (
                Type::Ieee { exponent, fraction },
                Type::Ieee {
                    exponent: rhs_exponent,
                    fraction: rhs_fraction,
                },
            ) => exponent
                .equivalent(*rhs_exponent, ctx)
                .and_then(|| fraction.equivalent(*rhs_fraction, ctx)),
            (
                Type::Record { name, args },
                Type::Record {
                    name: rhs_name,
                    args: rhs_args,
                },
            ) => {
                if name != rhs_name {
                    LBool::False
                } else {
                    args.iter()
                        .zip(rhs_args)
                        .lifted_all(|(lhs, rhs)| lhs.equivalent(*rhs, ctx))
                }
            }
            _ => LBool::False,
        }
    }
}

impl Equivalent for Index<ty::Expression> {
    type Context = TypeArenas;

    fn equivalent(self, rhs: Self, ctx: &Self::Context) -> LBool {
        if self == rhs {
            return LBool::True;
        }

        let solver = z3::Solver::new();

        solver.assert(self.into_smt(ctx).ne(rhs.into_smt(ctx)));

        match solver.check() {
            z3::SatResult::Unsat => LBool::True,
            z3::SatResult::Unknown => LBool::Unknown,
            z3::SatResult::Sat => LBool::False,
        }
    }
}

pub trait Entailed {
    type Context;

    fn entailed(self, asserts: &z3::Solver, ctx: &Self::Context) -> LBool;
}

impl<T: IntoSmt<Output = ast::Bool>> Entailed for T {
    type Context = <T as IntoSmt>::Context;

    fn entailed(self, asserts: &z3::Solver, ctx: &Self::Context) -> LBool {
        asserts.push();
        asserts.assert(!self.into_smt(ctx));

        let result = match asserts.check() {
            z3::SatResult::Unsat => LBool::True,
            z3::SatResult::Unknown => LBool::Unknown,
            z3::SatResult::Sat => LBool::False,
        };

        asserts.pop(1);

        result
    }
}

pub trait IntoSmt {
    type Output: ast::Ast;
    type Context;

    fn into_smt(self, ctx: &Self::Context) -> Self::Output;
}

impl IntoSmt for Index<ty::Expression> {
    type Output = ast::Real;
    type Context = TypeArenas;

    fn into_smt(self, ctx: &TypeArenas) -> ast::Real {
        match ctx[self] {
            ty::Expression::Param(i) => ast::Real::new_const(u32::from(i)),
            ty::Expression::Const(value) => {
                ast::Real::from_int(&ast::Int::from_u64(value))
            }
            ty::Expression::Neg(expr) => -expr.into_smt(ctx),
            ty::Expression::Binary(op, lhs, rhs) => {
                let lhs = lhs.into_smt(ctx);
                let rhs = rhs.into_smt(ctx);

                match op {
                    ty::BinaryOp::Add => lhs + rhs,
                    ty::BinaryOp::Sub => lhs - rhs,
                    ty::BinaryOp::Mul => lhs * rhs,
                    ty::BinaryOp::Div => lhs / rhs,
                    ty::BinaryOp::Pow => lhs.power(rhs),
                }
            }
        }
    }
}

impl IntoSmt for Index<ty::Proposition> {
    type Output = ast::Bool;
    type Context = TypeArenas;

    fn into_smt(self, ctx: &TypeArenas) -> ast::Bool {
        match ctx[self] {
            ty::Proposition::Not(prop) => !prop.into_smt(ctx),
            ty::Proposition::And(lhs, rhs) => {
                lhs.into_smt(ctx) & rhs.into_smt(ctx)
            }
            ty::Proposition::Or(lhs, rhs) => {
                lhs.into_smt(ctx) | rhs.into_smt(ctx)
            }
            ty::Proposition::Relation(p, lhs, rhs) => {
                let lhs = lhs.into_smt(ctx);
                let rhs = rhs.into_smt(ctx);

                match p {
                    ty::Predicate::Eq => lhs.eq(rhs),
                    ty::Predicate::Ne => lhs.ne(rhs),
                    ty::Predicate::Gt => lhs.gt(rhs),
                    ty::Predicate::Ge => lhs.ge(rhs),
                    ty::Predicate::Lt => lhs.lt(rhs),
                    ty::Predicate::Le => lhs.le(rhs),
                }
            }
        }
    }
}
