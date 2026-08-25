mod check;
mod errors;
mod flags;
mod printer;
mod promotion;
mod queries;
mod substitution;
mod types;
mod visit;

pub use check::check_hir;
pub use printer::{Pretty, Printer};
pub use types::{
    BinaryOp, Expression, Predicate, Proposition, Type, TypeArenas,
};
pub use visit::{Visitable, Visitor};
