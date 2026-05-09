use std::fmt;

use adpl_hir as hir;
use adpl_util::Diagnostic;

use crate::printer::{Pretty, Printer};
use crate::types::{Index, Type};

pub enum OpKind<'a> {
    Unary(&'a hir::UnaryOp),
    Binary(&'a hir::BinaryOp),
    Call(&'a hir::Call),
    Record(&'a hir::Constructor),
}

impl OpKind<'_> {
    fn span(&self) -> hir::Span {
        match self {
            OpKind::Unary(op) => op.span,
            OpKind::Binary(op) => op.span,
            OpKind::Call(call) => call.name.span,
            OpKind::Record(cons) => cons.name.span,
        }
    }
}

impl fmt::Display for OpKind<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OpKind::Unary(op) => write!(f, "operator `{}`", op.kind.as_str()),
            OpKind::Binary(op) => write!(f, "operator `{}`", op.kind.as_str()),
            OpKind::Call(call) => write!(f, "`{}`", call.name.symbol),
            OpKind::Record(_) => write!(f, "initializer"),
        }
    }
}

pub struct WarnNotChecked<'a> {
    pub what: &'a str,
    pub expr: &'a hir::Expression,
}

impl From<WarnNotChecked<'_>> for Diagnostic {
    fn from(value: WarnNotChecked) -> Self {
        Diagnostic::warning()
            .with_message(format!("{} are not yet checked", value.what))
            .with_primary(value.expr.span, "")
    }
}

pub struct WarnUnreachable<'a> {
    pub divergent: &'a hir::Statement,
    pub unreachable: &'a hir::Statement,
}

impl From<WarnUnreachable<'_>> for Diagnostic {
    fn from(value: WarnUnreachable) -> Self {
        Diagnostic::warning()
            .with_message("unreachable statement")
            .with_secondary(value.divergent.span, "execution ends here")
            .with_primary(value.unreachable.span, "unreachable statement")
    }
}

pub struct ExpectedConst<'a> {
    pub expr: &'a hir::Expression,
}

impl From<ExpectedConst<'_>> for Diagnostic {
    fn from(value: ExpectedConst) -> Self {
        Diagnostic::error()
            .with_message("use of non-constant value in a constant context")
            .with_primary(value.expr.span, "not a constant")
    }
}

pub struct NoDenotation<'a> {
    pub op: OpKind<'a>,
    pub context: &'a str,
}

impl From<NoDenotation<'_>> for Diagnostic {
    fn from(value: NoDenotation) -> Self {
        Diagnostic::error()
            .with_message(format!(
                "cannot infer denotation for {}",
                value.context,
            ))
            .with_primary(
                value.op.span(),
                format!("{} has no real-valued denotation", value.op),
            )
    }
}

pub struct NoMatchingUnaryOverload<'a> {
    pub op: &'a hir::UnaryOp,
    pub arg: Index<Type>,
    pub printer: &'a Printer<'a>,
}

impl From<NoMatchingUnaryOverload<'_>> for Diagnostic {
    fn from(value: NoMatchingUnaryOverload) -> Self {
        Diagnostic::error()
            .with_message(format!(
                "no matching overload for operator `{}`",
                value.op.kind.as_str(),
            ))
            .with_primary(
                value.op.span,
                format!(
                    "no overload matches the argument type `{}`",
                    value.arg.pretty(value.printer),
                ),
            )
    }
}

pub struct NoMatchingBinaryOverload<'a> {
    pub op: &'a hir::BinaryOp,
    pub lhs: Index<Type>,
    pub rhs: Index<Type>,
    pub printer: &'a Printer<'a>,
}

impl From<NoMatchingBinaryOverload<'_>> for Diagnostic {
    fn from(value: NoMatchingBinaryOverload) -> Self {
        Diagnostic::error()
            .with_message(format!(
                "no matching overload for operator `{}`",
                value.op.kind.as_str(),
            ))
            .with_primary(
                value.op.span,
                format!(
                    "no overload matches the argument types `{}`, `{}`",
                    value.lhs.pretty(value.printer),
                    value.rhs.pretty(value.printer),
                ),
            )
    }
}

pub struct IncompatibleTypes<'a> {
    pub expr: &'a hir::Expression,
    pub expected: Index<Type>,
    pub found: Index<Type>,
    pub certain: bool,
    pub printer: &'a Printer<'a>,
}

impl From<IncompatibleTypes<'_>> for Diagnostic {
    fn from(value: IncompatibleTypes) -> Self {
        Diagnostic::error()
            .with_message(if value.certain {
                "incompatible types"
            } else {
                "failed to prove type equivalence"
            })
            .with_primary(
                value.expr.span,
                format!(
                    "expected type `{}`, found `{}`",
                    value.expected.pretty(value.printer),
                    value.found.pretty(value.printer),
                ),
            )
    }
}

pub struct UnmetPrecondition<'a> {
    pub callee: &'a hir::Id,
    pub kind: &'a str,
    pub requires: &'a hir::Expression,
    pub certain: bool,
}

impl From<UnmetPrecondition<'_>> for Diagnostic {
    fn from(value: UnmetPrecondition) -> Self {
        Diagnostic::error()
            .with_message(if value.certain {
                "precondition not satisfied"
            } else {
                "failed to prove precondition"
            })
            .with_secondary(
                value.requires.span,
                format!("required by `{}`", value.callee.symbol),
            )
            .with_primary(
                value.callee.span,
                format!("precondition for {} not satisfied", value.kind),
            )
    }
}

pub struct TypeHasNoFields<'a> {
    pub ty: Index<Type>,
    pub field: &'a hir::Id,
    pub printer: &'a Printer<'a>,
}

impl From<TypeHasNoFields<'_>> for Diagnostic {
    fn from(value: TypeHasNoFields) -> Self {
        Diagnostic::error()
            .with_message(format!(
                "type `{}` has no fields",
                value.ty.pretty(value.printer),
            ))
            .with_primary(value.field.span, "no such field")
    }
}

pub struct UnexpectedField<'a> {
    pub ty: &'a hir::Id,
    pub field: &'a hir::Id,
}

impl From<UnexpectedField<'_>> for Diagnostic {
    fn from(value: UnexpectedField) -> Self {
        Diagnostic::error()
            .with_message(format!(
                "`{}` is not a field of type `{}`",
                value.field.symbol, value.ty.symbol,
            ))
            .with_primary(value.field.span, "no such field")
    }
}

pub struct MissingReturn<'a> {
    pub def: &'a hir::Id,
}

impl From<MissingReturn<'_>> for Diagnostic {
    fn from(value: MissingReturn) -> Self {
        Diagnostic::error()
            .with_message(format!(
                "function `{}` terminates without returning a value",
                value.def.symbol,
            ))
            .with_primary(
                value.def.span,
                "body ends without a `return` statement",
            )
    }
}
