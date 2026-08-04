use std::fmt;

use adpl_hir as hir;
use adpl_util::Diagnostic;

use crate::printer::{Pretty, Printer};
use crate::types::{Index, Type};

pub enum OpKind<'a> {
    Field(&'a hir::Id),
    Unary(&'a hir::UnaryOp),
    Binary(&'a hir::BinaryOp),
    Call(&'a hir::Call),
    Record(&'a hir::Constructor),
    If(&'a hir::Expression),
}

impl OpKind<'_> {
    fn span(&self) -> hir::Span {
        match self {
            OpKind::Field(field) => field.span,
            OpKind::Unary(op) => op.span,
            OpKind::Binary(op) => op.span,
            OpKind::Call(call) => call.name.span,
            OpKind::Record(cons) => cons.name.span,
            OpKind::If(expr) => expr.span,
        }
    }
}

impl fmt::Display for OpKind<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OpKind::Field(_) => write!(f, "field access"),
            OpKind::Unary(op) => write!(f, "operator `{}`", op.kind.as_str()),
            OpKind::Binary(op) => write!(f, "operator `{}`", op.kind.as_str()),
            OpKind::Call(call) => write!(f, "`{}`", call.name.symbol),
            OpKind::Record(_) => write!(f, "initializer"),
            OpKind::If(_) => write!(f, "conditional"),
        }
    }
}

pub struct ExpectedProposition<'a> {
    pub expr: &'a hir::Expression,
}

impl From<ExpectedProposition<'_>> for Diagnostic {
    fn from(value: ExpectedProposition) -> Self {
        Diagnostic::error()
            .with_message("expected proposition, found term")
            .with_primary(
                value.expr.span,
                "term may not be used as a proposition",
            )
    }
}

pub struct DeclaredTypeNotRealValued<'a> {
    pub name: &'a hir::Id,
    pub ty: &'a hir::Type,
}

impl From<DeclaredTypeNotRealValued<'_>> for Diagnostic {
    fn from(value: DeclaredTypeNotRealValued) -> Self {
        Diagnostic::error()
            .with_message(format!(
                "real `{}` has non-real type",
                value.name.symbol,
            ))
            .with_primary(value.ty.span, "expected a real-valued type")
    }
}

pub struct TypeNotRealValued<'a> {
    pub expr: &'a hir::Expression,
    pub ty: Index<Type>,
    pub printer: &'a Printer<'a>,
}

impl From<TypeNotRealValued<'_>> for Diagnostic {
    fn from(value: TypeNotRealValued) -> Self {
        Diagnostic::error()
            .with_message("expected a real-valued type")
            .with_primary(
                value.expr.span,
                format!(
                    "type `{}` not real-valued",
                    value.ty.pretty(value.printer),
                ),
            )
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

pub struct IncompatibleBranches<'a> {
    pub then_expr: &'a hir::Expression,
    pub else_expr: &'a hir::Expression,
    pub then_ty: Index<Type>,
    pub else_ty: Index<Type>,
    pub printer: &'a Printer<'a>,
}

impl From<IncompatibleBranches<'_>> for Diagnostic {
    fn from(value: IncompatibleBranches) -> Self {
        Diagnostic::error()
            .with_message("branches have incompatible types")
            .with_secondary(
                value.then_expr.span,
                format!(
                    "this has type `{}`",
                    value.then_ty.pretty(value.printer),
                ),
            )
            .with_primary(
                value.else_expr.span,
                format!(
                    "expected type `{}`, found `{}`",
                    value.then_ty.pretty(value.printer),
                    value.else_ty.pretty(value.printer),
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

pub struct AssertionFailed<'a> {
    pub assert: &'a hir::Statement,
    pub certain: bool,
}

impl From<AssertionFailed<'_>> for Diagnostic {
    fn from(value: AssertionFailed) -> Self {
        Diagnostic::error()
            .with_message(if value.certain {
                "assertion does not hold"
            } else {
                "failed to prove assertion"
            })
            .with_primary(value.assert.span, "assertion failed")
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
