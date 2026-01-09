use adpl_ast as ast;
use adpl_util::Diagnostic;

pub struct ReusedParameter<'a> {
    pub second: &'a ast::Id,
}

impl From<ReusedParameter<'_>> for Diagnostic {
    fn from(value: ReusedParameter) -> Self {
        Diagnostic::error()
            .with_message(format!(
                "reuse of parameter name `{}`",
                value.second.symbol,
            ))
            .with_primary(value.second.span, "parameter name already used")
    }
}

pub struct ShadowedGeneric<'a> {
    pub first: &'a ast::Id,
    pub second: &'a ast::Id,
}

impl From<ShadowedGeneric<'_>> for Diagnostic {
    fn from(value: ShadowedGeneric) -> Self {
        Diagnostic::error()
            .with_message(format!(
                "parameter `{}` shadows a generic parameter",
                value.second.symbol,
            ))
            .with_secondary(value.first.span, "name first used here")
            .with_primary(value.second.span, "shadows a generic parameter")
    }
}

pub struct RedeclaredField<'a> {
    pub first: &'a ast::Id,
    pub second: &'a ast::Id,
}

impl From<RedeclaredField<'_>> for Diagnostic {
    fn from(value: RedeclaredField) -> Self {
        Diagnostic::error()
            .with_message(format!(
                "reuse of field name `{}`",
                value.first.symbol,
            ))
            .with_secondary(value.first.span, "field first declared here")
            .with_primary(value.second.span, "field name already used")
    }
}

pub struct RedefinedName<'a> {
    pub first: &'a ast::Id,
    pub second: &'a ast::Id,
}

impl From<RedefinedName<'_>> for Diagnostic {
    fn from(value: RedefinedName) -> Self {
        Diagnostic::error()
            .with_message(format!(
                "redefinition of name `{}`",
                value.first.symbol,
            ))
            .with_secondary(value.first.span, "name first defined here")
            .with_primary(value.second.span, "name already defined")
    }
}

pub struct UndefinedName<'a> {
    pub name: &'a ast::Id,
}

impl From<UndefinedName<'_>> for Diagnostic {
    fn from(value: UndefinedName) -> Self {
        Diagnostic::error()
            .with_message(format!("undefined name `{}`", value.name.symbol))
            .with_primary(value.name.span, "undefined name")
    }
}

pub struct KindNotFound<'a> {
    pub name: &'a ast::Id,
    pub kind: &'a str,
}

impl From<KindNotFound<'_>> for Diagnostic {
    fn from(value: KindNotFound) -> Self {
        Diagnostic::error()
            .with_message(format!(
                "no {} with name `{}`",
                value.kind, value.name.symbol,
            ))
            .with_primary(value.name.span, format!("{} not found", value.kind))
    }
}

pub struct UnexpectedItem<'a> {
    pub name: &'a ast::Id,
    pub kind: &'a str,
}

impl From<UnexpectedItem<'_>> for Diagnostic {
    fn from(value: UnexpectedItem) -> Self {
        Diagnostic::error()
            .with_message(format!("expected value, found {}", value.kind))
            .with_primary(value.name.span, "expected value")
    }
}

pub struct UnexpectedKind<'a> {
    pub name: &'a ast::Id,
    pub expected: &'a str,
    pub found: &'a str,
    pub label: &'a str,
}

impl From<UnexpectedKind<'_>> for Diagnostic {
    fn from(value: UnexpectedKind) -> Self {
        Diagnostic::error()
            .with_message(format!(
                "expected {}, found {}",
                value.expected, value.found,
            ))
            .with_primary(value.name.span, value.label)
    }
}

pub struct ArityMismatch<'a> {
    pub callee: &'a ast::Id,
    pub expected: usize,
    pub found: usize,
    pub what: &'a str,
}

impl From<ArityMismatch<'_>> for Diagnostic {
    fn from(value: ArityMismatch) -> Self {
        let expected = format!(
            "{} {}{}",
            value.expected,
            value.what,
            if value.expected == 1 { "" } else { "s" },
        );

        Diagnostic::error()
            .with_message(format!(
                "`{}` takes {}, found {}",
                value.callee.symbol, expected, value.found,
            ))
            .with_primary(value.callee.span, format!("expected {}", expected))
    }
}

pub struct UnexpectedField<'a> {
    pub ty: &'a ast::Id,
    pub field: &'a ast::Id,
}

impl From<UnexpectedField<'_>> for Diagnostic {
    fn from(value: UnexpectedField) -> Self {
        Diagnostic::error()
            .with_message(format!(
                "`{}` is not a field of `{}`",
                value.field.symbol, value.ty.symbol,
            ))
            .with_primary(value.field.span, "no such field")
    }
}

pub struct DuplicateField<'a> {
    pub second: &'a ast::Id,
}

impl From<DuplicateField<'_>> for Diagnostic {
    fn from(value: DuplicateField) -> Self {
        Diagnostic::error()
            .with_message(format!(
                "field `{}` specified more than once",
                value.second.symbol,
            ))
            .with_primary(value.second.span, "field already initialized")
    }
}

pub struct MissingField<'a> {
    pub ty: &'a ast::Id,
    pub field: &'a ast::Id,
}

impl From<MissingField<'_>> for Diagnostic {
    fn from(value: MissingField) -> Self {
        Diagnostic::error()
            .with_message(format!(
                "missing field `{}` in initializer for `{}`",
                value.field.symbol, value.ty.symbol,
            ))
            .with_primary(value.ty.span, "incomplete initializer")
    }
}
