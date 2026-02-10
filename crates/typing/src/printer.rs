use std::fmt;

use adpl_arena::{Index, IndexRange};
use adpl_hir as hir;

use crate::types as ty;

pub trait Pretty: Copy {
    type Printer<'a>;

    fn write_pretty(
        self,
        p: &Self::Printer<'_>,
        w: &mut dyn fmt::Write,
    ) -> fmt::Result;

    fn pretty(self, p: &Self::Printer<'_>) -> impl fmt::Display {
        struct Display<'a, 'b, P: Pretty> {
            value: P,
            p: &'a P::Printer<'b>,
        }

        impl<P: Pretty> fmt::Display for Display<'_, '_, P> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                self.value.write_pretty(self.p, f)
            }
        }

        Display { value: self, p }
    }
}

impl Pretty for Index<ty::Type> {
    type Printer<'a> = Printer<'a>;

    fn write_pretty(
        self,
        p: &Self::Printer<'_>,
        w: &mut dyn fmt::Write,
    ) -> fmt::Result {
        match p.types[self] {
            ty::Type::Real => write!(w, "real"),
            ty::Type::UnsizedInteger => write!(w, "integer"),
            ty::Type::Bool => write!(w, "bool"),
            ty::Type::Record { name, ref args } => {
                write!(w, "{}", p.hir[name].name.symbol)?;

                match args.as_ref() {
                    [] => {}
                    [first, rest @ ..] => {
                        write!(w, "[")?;
                        first.write_pretty(p, w)?;

                        for arg in rest {
                            write!(w, ", ")?;
                            arg.write_pretty(p, w)?;
                        }

                        write!(w, "]")?;
                    }
                }

                Ok(())
            }
        }
    }
}

impl Pretty for Index<ty::Expression> {
    type Printer<'a> = Printer<'a>;

    fn write_pretty(
        self,
        p: &Self::Printer<'_>,
        w: &mut dyn fmt::Write,
    ) -> fmt::Result {
        p.print(self, w)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Direction {
    Left,
    Right,
}

type Associativity = Direction;

impl ty::BinaryOp {
    fn precedence(self) -> (u8, Associativity) {
        match self {
            ty::BinaryOp::Add | ty::BinaryOp::Sub => (1, Associativity::Left),
            ty::BinaryOp::Mul | ty::BinaryOp::Div => (2, Associativity::Left),
            ty::BinaryOp::Pow => (4, Associativity::Right),
        }
    }

    fn pretty(self) -> &'static str {
        match self {
            ty::BinaryOp::Add => " + ",
            ty::BinaryOp::Sub => " - ",
            ty::BinaryOp::Mul => " * ",
            ty::BinaryOp::Div => " / ",
            ty::BinaryOp::Pow => "^",
        }
    }
}

pub struct Printer<'a> {
    pub hir: &'a hir::Context,
    pub types: &'a ty::TypeArenas,
    pub params: IndexRange<hir::Local>,
}

impl<'a> Printer<'a> {
    pub fn new(
        hir: &'a hir::Context,
        types: &'a ty::TypeArenas,
        params: IndexRange<hir::Local>,
    ) -> Self {
        Printer { hir, types, params }
    }

    fn print(
        &self,
        expr: Index<ty::Expression>,
        w: &mut dyn fmt::Write,
    ) -> fmt::Result {
        self.print_child(expr, Direction::Left, 0, w)
    }

    fn print_child(
        &self,
        child: Index<ty::Expression>,
        side: Direction,
        parent: u8,
        w: &mut dyn fmt::Write,
    ) -> fmt::Result {
        match self.types[child] {
            ty::Expression::Param(i) => {
                let param = &self.hir[self.params][usize::from(i)];

                write!(w, "{}", param.name.symbol)
            }
            ty::Expression::Const(value) => write!(w, "{value}"),
            ty::Expression::Neg(expr) => {
                const PRECEDENCE: u8 = 3;

                if parent > PRECEDENCE && side == Direction::Left {
                    write!(w, "(-")?;
                    self.print_child(expr, Direction::Right, PRECEDENCE, w)?;
                    write!(w, ")")
                } else {
                    write!(w, "-")?;
                    self.print_child(expr, Direction::Right, PRECEDENCE, w)
                }
            }
            ty::Expression::Binary(op, lhs, rhs) => {
                let (precedence, assoc) = op.precedence();

                let parenthesize = parent > precedence
                    || (parent == precedence && assoc != side);

                if parenthesize {
                    write!(w, "(")?;
                }

                self.print_child(lhs, Direction::Left, precedence, w)?;
                write!(w, "{}", op.pretty())?;
                self.print_child(rhs, Direction::Right, precedence, w)?;

                if parenthesize {
                    write!(w, ")")?;
                }

                Ok(())
            }
        }
    }
}
