mod check;
mod errors;
mod printer;
mod promotion;
mod substitution;
mod types;

pub use check::check_hir;
pub use printer::{Pretty, Printer};
pub use types::{BinaryOp, Expression, Type, TypeArenas};
