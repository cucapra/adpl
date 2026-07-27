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
            ty::Type::Integer => write!(w, "integer"),
            ty::Type::Bool => write!(w, "bool"),
            ty::Type::Int(width) => write!(w, "int[{}]", width.pretty(p)),
            ty::Type::UInt(width) => write!(w, "uint[{}]", width.pretty(p)),
            ty::Type::Ieee {
                exponent: e,
                fraction: f,
            } => write!(w, "ieee[{}, {}]", e.pretty(p), f.pretty(p)),
            ty::Type::Record { name, ref args } => {
                write!(w, "{}", p.hir[name].name.symbol)?;

                match args.as_ref() {
                    [] => {}
                    [first, rest @ ..] => {
                        write!(w, "[{}", first.pretty(p))?;

                        for arg in rest {
                            write!(w, ", {}", arg.pretty(p))?;
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
    pub item: Index<hir::Item>,
}

impl<'a> Printer<'a> {
    pub fn new(
        hir: &'a hir::Context,
        types: &'a ty::TypeArenas,
        item: Index<hir::Item>,
    ) -> Self {
        Printer { hir, types, item }
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
                let param =
                    &self.hir[self.hir[self.params()][usize::from(i)].local];

                write!(w, "{}", param.name.symbol)
            }
            ty::Expression::GenericParam(i) => {
                let param = &self.hir[self.generics()][usize::from(i)];

                write!(w, "{}", param.name.symbol)
            }
            ty::Expression::Term(_) => write!(w, "..."),
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

    fn params(&self) -> IndexRange<hir::Parameter> {
        match self.hir[self.item] {
            hir::Item::Record(_) => panic!(),
            hir::Item::Def(def) => self.hir[def].inputs,
        }
    }

    fn generics(&self) -> IndexRange<hir::Local> {
        match self.hir[self.item] {
            hir::Item::Record(record) => self.hir[record].params,
            hir::Item::Def(def) => self.hir[def].generics,
        }
    }
}
