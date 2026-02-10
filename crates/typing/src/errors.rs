use adpl_hir as hir;
use adpl_util::Diagnostic;

use crate::printer::{Pretty, Printer};
use crate::types::{Index, Type};

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

pub struct NonConstInGeneric<'a> {
    pub expr: &'a hir::Expression,
    pub secondary: Option<&'a hir::Expression>,
}

impl From<NonConstInGeneric<'_>> for Diagnostic {
    fn from(value: NonConstInGeneric) -> Self {
        let error = Diagnostic::error()
            .with_message("use of non-constant value in a constant context")
            .with_primary(value.expr.span, "not a constant");

        if let Some(expr) = value.secondary {
            error.with_secondary(expr.span, "used here in a constant context")
        } else {
            error
        }
    }
}

pub struct NotAllowedInGeneric<'a> {
    pub what: &'a str,
    pub primary: hir::Span,
    pub secondary: Option<&'a hir::Expression>,
}

impl From<NotAllowedInGeneric<'_>> for Diagnostic {
    fn from(value: NotAllowedInGeneric) -> Self {
        let error = Diagnostic::error()
            .with_message(format!(
                "{} not allowed in generic argument expression",
                value.what,
            ))
            .with_primary(value.primary, "not allowed here");

        if let Some(expr) = value.secondary {
            error.with_secondary(expr.span, "not allowed because of this use")
        } else {
            error
        }
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
    pub printer: &'a Printer<'a>,
}

impl From<IncompatibleTypes<'_>> for Diagnostic {
    fn from(value: IncompatibleTypes) -> Self {
        Diagnostic::error()
            .with_message("incompatible types")
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
