use std::convert::Infallible;
use std::ops::ControlFlow;

use crate::context::Context;
use crate::hir::{self, Index, List};

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

impl<E> VisitResult for Result<(), E> {
    type Residual = E;

    fn output() -> Self {
        Ok(())
    }

    fn from_residual(residual: E) -> Self {
        Err(residual)
    }

    fn branch(self) -> ControlFlow<E> {
        match self {
            Ok(()) => ControlFlow::Continue(()),
            Err(err) => ControlFlow::Break(err),
        }
    }
}

pub trait Visitor<'ctx> {
    type Result: VisitResult;

    fn ctx(&self) -> &'ctx Context;

    fn visit_item(&mut self, item: Index<hir::Item>) -> Self::Result {
        visit_item(self, item)
    }

    fn visit_record(&mut self, record: Index<hir::Record>) -> Self::Result {
        visit_record(self, record)
    }

    fn visit_field(&mut self, field: Index<hir::Field>) -> Self::Result {
        visit_field(self, field)
    }

    fn visit_type(&mut self, ty: Index<hir::Type>) -> Self::Result {
        visit_type(self, ty)
    }

    fn visit_generic_argument(
        &mut self,
        expr: Index<hir::Expression>,
    ) -> Self::Result {
        self.visit_expression(expr)
    }

    fn visit_definition(
        &mut self,
        def: Index<hir::Definition>,
    ) -> Self::Result {
        visit_definition(self, def)
    }

    fn visit_parameter(
        &mut self,
        param: Index<hir::Parameter>,
    ) -> Self::Result {
        visit_parameter(self, param)
    }

    fn visit_block(&mut self, block: List<hir::Statement>) -> Self::Result {
        visit_block(self, block)
    }

    fn visit_statement(&mut self, stmt: Index<hir::Statement>) -> Self::Result {
        visit_statement(self, stmt)
    }

    fn visit_proposition(
        &mut self,
        expr: Index<hir::Expression>,
    ) -> Self::Result {
        self.visit_expression(expr)
    }

    fn visit_denotation(
        &mut self,
        expr: Index<hir::Expression>,
    ) -> Self::Result {
        self.visit_expression(expr)
    }

    fn visit_expression(
        &mut self,
        expr: Index<hir::Expression>,
    ) -> Self::Result {
        visit_expression(self, expr)
    }

    fn visit_identifier_expr(
        &mut self,
        _local: Index<hir::Local>,
        _span: hir::Span,
    ) -> Self::Result {
        VisitResult::output()
    }

    fn visit_literal_expr(
        &mut self,
        _lit: &'ctx hir::Literal,
        _span: hir::Span,
    ) -> Self::Result {
        VisitResult::output()
    }
}

macro_rules! try_visit {
    ($e:expr) => {
        match $e.branch() {
            ControlFlow::Continue(()) => {}
            ControlFlow::Break(residual) => {
                return VisitResult::from_residual(residual);
            }
        }
    };
}

pub fn visit_item<'ctx, V: Visitor<'ctx> + ?Sized>(
    visitor: &mut V,
    item: Index<hir::Item>,
) -> V::Result {
    match visitor.ctx()[item] {
        hir::Item::Record(record) => visitor.visit_record(record),
        hir::Item::Def(def) => visitor.visit_definition(def),
    }
}

pub fn visit_record<'ctx, V: Visitor<'ctx> + ?Sized>(
    visitor: &mut V,
    record: Index<hir::Record>,
) -> V::Result {
    let record = &visitor.ctx()[record];

    if let Some(expr) = record.requires {
        try_visit!(visitor.visit_proposition(expr.get()));
    }

    for field in record.fields {
        try_visit!(visitor.visit_field(field));
    }

    VisitResult::output()
}

pub fn visit_field<'ctx, V: Visitor<'ctx> + ?Sized>(
    visitor: &mut V,
    field: Index<hir::Field>,
) -> V::Result {
    visitor.visit_type(visitor.ctx()[field].ty)
}

pub fn visit_type<'ctx, V: Visitor<'ctx> + ?Sized>(
    visitor: &mut V,
    ty: Index<hir::Type>,
) -> V::Result {
    for &expr in &visitor.ctx()[visitor.ctx()[ty].args] {
        try_visit!(visitor.visit_generic_argument(expr));
    }

    VisitResult::output()
}

pub fn visit_definition<'ctx, V: Visitor<'ctx> + ?Sized>(
    visitor: &mut V,
    def: Index<hir::Definition>,
) -> V::Result {
    let def = &visitor.ctx()[def];

    for param in def.inputs {
        try_visit!(visitor.visit_parameter(param));
    }

    try_visit!(visitor.visit_type(def.output));

    if let Some(expr) = def.requires {
        try_visit!(visitor.visit_proposition(expr.get()));
    }

    if let Some(expr) = def.implements {
        try_visit!(visitor.visit_denotation(expr.get()));
    }

    if let Some(block) = def.body {
        visitor.visit_block(block)
    } else {
        VisitResult::output()
    }
}

pub fn visit_parameter<'ctx, V: Visitor<'ctx> + ?Sized>(
    visitor: &mut V,
    param: Index<hir::Parameter>,
) -> V::Result {
    visitor.visit_type(visitor.ctx()[param].ty)
}

pub fn visit_block<'ctx, V: Visitor<'ctx> + ?Sized>(
    visitor: &mut V,
    block: List<hir::Statement>,
) -> V::Result {
    for &stmt in &visitor.ctx()[block] {
        try_visit!(visitor.visit_statement(stmt));
    }

    VisitResult::output()
}

pub fn visit_statement<'ctx, V: Visitor<'ctx> + ?Sized>(
    visitor: &mut V,
    stmt: Index<hir::Statement>,
) -> V::Result {
    match visitor.ctx()[stmt].kind {
        hir::StmtKind::Let(_, expr) => visitor.visit_expression(expr),
        hir::StmtKind::Real(_, expr) => visitor.visit_expression(expr),
        hir::StmtKind::Assert(expr) => visitor.visit_proposition(expr),
        hir::StmtKind::Return(expr) => visitor.visit_expression(expr),
        hir::StmtKind::Unsafe(block) => visitor.visit_block(block),
    }
}

pub fn visit_expression<'ctx, V: Visitor<'ctx> + ?Sized>(
    visitor: &mut V,
    expr: Index<hir::Expression>,
) -> V::Result {
    let expr = &visitor.ctx()[expr];

    match expr.kind {
        hir::ExprKind::Id(local) => {
            visitor.visit_identifier_expr(local, expr.span)
        }
        hir::ExprKind::Lit(ref lit) => {
            visitor.visit_literal_expr(lit, expr.span)
        }
        hir::ExprKind::Field(expr, _) => visitor.visit_expression(expr),
        hir::ExprKind::Unary(_, expr) => visitor.visit_expression(expr),
        hir::ExprKind::Binary(_, lhs, rhs) => {
            try_visit!(visitor.visit_expression(lhs));

            visitor.visit_expression(rhs)
        }
        hir::ExprKind::Call(ref call) => {
            for &expr in &visitor.ctx()[call.generics] {
                try_visit!(visitor.visit_generic_argument(expr));
            }

            for &expr in &visitor.ctx()[call.args] {
                try_visit!(visitor.visit_expression(expr));
            }

            VisitResult::output()
        }
        hir::ExprKind::Record(ref cons) => {
            for &expr in &visitor.ctx()[cons.generics] {
                try_visit!(visitor.visit_generic_argument(expr));
            }

            for &expr in &visitor.ctx()[cons.inits] {
                try_visit!(visitor.visit_expression(expr));
            }

            VisitResult::output()
        }
        hir::ExprKind::If(cond, then, else_) => {
            try_visit!(visitor.visit_expression(cond));
            try_visit!(visitor.visit_expression(then));

            visitor.visit_expression(else_)
        }
    }
}
