use std::cell::OnceCell;
use std::collections::HashMap;
use std::ops;

use adpl_arena::{DenseMap, Index, NonMaxIndex};
use adpl_hir as hir;
use adpl_util::{Diagnostic, Reporter};

use crate::printer::Printer;
use crate::promotion::Overloaded;
use crate::queries::{Entailed, Equivalent, IntoSmt, LBool};
use crate::substitution::{Arguments, Foldable};
use crate::{errors, types as ty};

pub fn check_hir(
    hir: &hir::Context,
    reporter: &mut Reporter,
) -> Option<TypingContext> {
    let mut tcx = TypingContext::with_default_env(hir);
    let mut lowering = LoweringContext::default();

    for (index, item) in hir.items.iter() {
        match *item {
            hir::Item::Record(record) => {
                let mut ctx = ItemContext {
                    hir,
                    reporter,
                    tcx: &mut tcx,
                    lowering: &mut lowering,
                    asserts: OnceCell::new(),
                    item: index,
                };

                ctx.check_record(record).ok()?;
            }
            hir::Item::Def(def) => {
                lowering.clear();

                let mut ctx = DefinitionContext {
                    icx: ItemContext {
                        hir,
                        reporter,
                        tcx: &mut tcx,
                        lowering: &mut lowering,
                        asserts: OnceCell::new(),
                        item: index,
                    },
                    def,
                };

                ctx.check_definition().ok()?;
            }
        }
    }

    Some(tcx)
}

#[derive(Debug)]
struct TypingError;

type Result<T> = std::result::Result<T, TypingError>;

pub struct Record {
    pub requires: Option<NonMaxIndex<ty::Proposition>>,
    pub fields: Box<[Index<ty::Type>]>,
}

pub struct Signature {
    pub requires: Option<NonMaxIndex<ty::Proposition>>,
    pub inputs: Box<[Index<ty::Type>]>,
    pub output: Index<ty::Type>,
}

pub struct TypingContext {
    pub arenas: ty::TypeArenas,
    pub records: DenseMap<hir::Record, Record>,
    pub signatures: DenseMap<hir::Definition, Signature>,
    pub env: DenseMap<hir::Local, Index<ty::Type>>,
}

impl TypingContext {
    fn with_default_env(hir: &hir::Context) -> Self {
        TypingContext {
            arenas: ty::TypeArenas::new(),
            records: DenseMap::new(),
            signatures: DenseMap::new(),
            env: DenseMap::filled(hir.locals.len(), Index::ZERO),
        }
    }
}

#[derive(Default)]
struct LoweringContext {
    reals: HashMap<Index<hir::Local>, Index<ty::Expression>>,
    context: Option<&'static str>,
}

impl LoweringContext {
    fn clear(&mut self) {
        self.reals.clear();
    }
}

struct ItemContext<'a, 'src> {
    hir: &'a hir::Context,
    reporter: &'a mut Reporter<'src>,
    tcx: &'a mut TypingContext,
    lowering: &'a mut LoweringContext,
    asserts: OnceCell<z3::Solver>,
    item: Index<hir::Item>,
}

impl ItemContext<'_, '_> {
    fn lower_type(&mut self, ty: Index<hir::Type>) -> Result<Index<ty::Type>> {
        let ty = &self.hir[ty];
        let args = &self.hir[ty.args];

        let ty = match ty.kind {
            hir::TypeKind::Bool => self.tcx.arenas.prims.bool,
            hir::TypeKind::Int => {
                let width = self.lower_generic_argument(args[0])?;

                self.tcx.arenas.intern(ty::Type::Int(width))
            }
            hir::TypeKind::UInt => {
                let width = self.lower_generic_argument(args[0])?;

                self.tcx.arenas.intern(ty::Type::UInt(width))
            }
            hir::TypeKind::Ieee => {
                let exponent = self.lower_generic_argument(args[0])?;
                let fraction = self.lower_generic_argument(args[1])?;

                self.tcx
                    .arenas
                    .intern(ty::Type::Ieee { exponent, fraction })
            }
            hir::TypeKind::Record(name) => {
                let args = args
                    .iter()
                    .map(|&expr| self.lower_generic_argument(expr))
                    .collect::<Result<_>>()?;

                self.tcx.arenas.intern(ty::Type::Record { name, args })
            }
        };

        Ok(ty)
    }

    fn lower_generic_argument(
        &mut self,
        expr: Index<hir::Expression>,
    ) -> Result<Index<ty::Expression>> {
        let found = self.check_expression(expr)?;
        let expected = self.tcx.arenas.prims.integer;

        if found != expected {
            self.reporter
                .emit(Diagnostic::from(errors::IncompatibleTypes {
                    expr: &self.hir[expr],
                    expected,
                    found,
                    certain: true,
                    printer: &self.printer(),
                }));

            return Err(TypingError);
        }

        self.within("generic argument").lower_expression(expr)
    }

    fn lower_expression(
        &mut self,
        expr: Index<hir::Expression>,
    ) -> Result<Index<ty::Expression>> {
        match self.hir[expr].kind {
            hir::ExprKind::Id(local) => match self.hir[local].kind {
                hir::LocalKind::Let(expr) => {
                    let ty = self.tcx.env[local];

                    if !self.tcx.arenas[ty].is_real_valued() {
                        self.reporter.emit(Diagnostic::from(
                            errors::TypeNotRealValued {
                                expr: &self.hir[expr],
                                ty,
                                printer: &self.printer(),
                            },
                        ));

                        return Err(TypingError);
                    }

                    Ok(self.tcx.arenas.intern(ty::Expression::Term(expr)))
                }
                hir::LocalKind::Real(_) => Ok(self.lowering.reals[&local]),
                hir::LocalKind::Param(i) => {
                    let ty = self.tcx.env[local];

                    if !self.tcx.arenas[ty].is_real_valued() {
                        self.reporter.emit(Diagnostic::from(
                            errors::TypeNotRealValued {
                                expr: &self.hir[expr],
                                ty,
                                printer: &self.printer(),
                            },
                        ));

                        return Err(TypingError);
                    }

                    Ok(self.tcx.arenas.intern(ty::Expression::Param(i)))
                }
                hir::LocalKind::GenericParam(i) => {
                    Ok(self.tcx.arenas.intern(ty::Expression::GenericParam(i)))
                }
            },
            hir::ExprKind::Lit(ref literal) => {
                Ok(self.tcx.arenas.intern(ty::Expression::Const(literal.value)))
            }
            hir::ExprKind::Field(_, ref field) => {
                self.reporter.emit(errors::NoDenotation {
                    op: errors::OpKind::Field(field),
                    context: self.lowering.context.unwrap(),
                });

                Err(TypingError)
            }
            hir::ExprKind::Unary(ref op, arg) => match op.kind {
                hir::UnaryKind::Neg => {
                    let arg = self.lower_expression(arg)?;

                    Ok(self.tcx.arenas.intern(ty::Expression::Neg(arg)))
                }
                hir::UnaryKind::Not => {
                    Err(self.lower_expression(arg).unwrap_err())
                }
            },
            hir::ExprKind::Binary(ref op, lhs, rhs) => {
                let lhs = self.lower_expression(lhs)?;
                let rhs = self.lower_expression(rhs)?;

                let Ok(op) = op.kind.try_into() else {
                    self.reporter.emit(errors::NoDenotation {
                        op: errors::OpKind::Binary(op),
                        context: self.lowering.context.unwrap(),
                    });

                    return Err(TypingError);
                };

                Ok(self.tcx.arenas.intern(ty::Expression::Binary(op, lhs, rhs)))
            }
            hir::ExprKind::Call(ref call) => {
                self.reporter.emit(errors::NoDenotation {
                    op: errors::OpKind::Call(call),
                    context: self.lowering.context.unwrap(),
                });

                Err(TypingError)
            }
            hir::ExprKind::Record(ref cons) => {
                self.reporter.emit(errors::NoDenotation {
                    op: errors::OpKind::Record(cons),
                    context: self.lowering.context.unwrap(),
                });

                Err(TypingError)
            }
        }
    }

    fn lower_precondition(
        &mut self,
        expr: Index<hir::Expression>,
    ) -> Result<Index<ty::Proposition>> {
        let found = self.check_expression(expr)?;
        let expected = self.tcx.arenas.prims.bool;

        if found != expected {
            self.reporter
                .emit(Diagnostic::from(errors::IncompatibleTypes {
                    expr: &self.hir[expr],
                    expected,
                    found,
                    certain: true,
                    printer: &self.printer(),
                }));

            return Err(TypingError);
        }

        self.within("precondition").lower_proposition(expr)
    }

    fn lower_proposition(
        &mut self,
        expr: Index<hir::Expression>,
    ) -> Result<Index<ty::Proposition>> {
        match self.hir[expr].kind {
            hir::ExprKind::Id(local) => match self.hir[local].kind {
                hir::LocalKind::Let(_) | hir::LocalKind::Param(_) => {
                    self.reporter.emit(errors::ExpectedProposition {
                        expr: &self.hir[expr],
                    });

                    Err(TypingError)
                }
                hir::LocalKind::Real(_) => unreachable!(),
                hir::LocalKind::GenericParam(_) => unreachable!(),
            },
            hir::ExprKind::Lit(_) => unreachable!(),
            hir::ExprKind::Field(..) => {
                self.reporter.emit(errors::ExpectedProposition {
                    expr: &self.hir[expr],
                });

                Err(TypingError)
            }
            hir::ExprKind::Unary(ref op, arg) => match op.kind {
                hir::UnaryKind::Neg => unreachable!(),
                hir::UnaryKind::Not => {
                    let arg = self.lower_proposition(arg)?;

                    Ok(self.tcx.arenas.intern(ty::Proposition::Not(arg)))
                }
            },
            hir::ExprKind::Binary(ref op, lhs, rhs) => match op.kind {
                hir::BinaryKind::And => {
                    let lhs = self.lower_proposition(lhs)?;
                    let rhs = self.lower_proposition(rhs)?;

                    Ok(self.tcx.arenas.intern(ty::Proposition::And(lhs, rhs)))
                }
                hir::BinaryKind::Or => {
                    let lhs = self.lower_proposition(lhs)?;
                    let rhs = self.lower_proposition(rhs)?;

                    Ok(self.tcx.arenas.intern(ty::Proposition::Or(lhs, rhs)))
                }
                kind => {
                    let lhs = self.lower_expression(lhs)?;
                    let rhs = self.lower_expression(rhs)?;

                    let op = kind.try_into().unwrap();

                    Ok(self
                        .tcx
                        .arenas
                        .intern(ty::Proposition::Relation(op, lhs, rhs)))
                }
            },
            hir::ExprKind::Call(_) => {
                self.reporter.emit(errors::ExpectedProposition {
                    expr: &self.hir[expr],
                });

                Err(TypingError)
            }
            hir::ExprKind::Record(_) => unreachable!(),
        }
    }

    fn check_record(&mut self, record: Index<hir::Record>) -> Result<()> {
        let (index, record) = (record, &self.hir[record]);

        for param in record.params {
            self.tcx.env[param] = self.tcx.arenas.prims.integer;
        }

        let requires = record
            .requires
            .map(|expr| {
                self.lower_precondition(expr.get())
                    .map(|index| index.try_into().unwrap())
            })
            .transpose()?;

        let fields = self.hir[record.fields]
            .iter()
            .map(|field| self.lower_type(field.ty))
            .collect::<Result<_>>()?;

        self.tcx
            .records
            .insert_back(index, Record { requires, fields });

        Ok(())
    }

    fn check_signature(&mut self, def: Index<hir::Definition>) -> Result<()> {
        let (index, def) = (def, &self.hir[def]);

        for param in def.generics {
            self.tcx.env[param] = self.tcx.arenas.prims.integer;
        }

        for param in &self.hir[def.inputs] {
            let ty = self.lower_type(param.ty)?;

            if param.modifier == hir::Modifier::Real
                && !self.tcx.arenas[ty].is_real_valued()
            {
                self.reporter.emit(errors::DeclaredTypeNotRealValued {
                    name: &self.hir[param.local].name,
                    ty: &self.hir[param.ty],
                });

                return Err(TypingError);
            }

            self.tcx.env[param.local] = ty;
        }

        let inputs = self.hir[def.inputs]
            .iter()
            .map(|param| self.tcx.env[param.local])
            .collect();

        let requires = def
            .requires
            .map(|expr| {
                let prop = self.lower_precondition(expr.get())?;

                self.asserts().assert(prop.into_smt(&self.tcx.arenas));

                Ok(prop.try_into().unwrap())
            })
            .transpose()?;

        let output = self.lower_type(def.output)?;

        let signature = Signature {
            requires,
            inputs,
            output,
        };

        self.tcx.signatures.insert_back(index, signature);

        Ok(())
    }

    fn check_expression(
        &mut self,
        expr: Index<hir::Expression>,
    ) -> Result<Index<ty::Type>> {
        let ty = match self.hir[expr].kind {
            hir::ExprKind::Id(local) => self.tcx.env[local],
            hir::ExprKind::Lit(_) => self.tcx.arenas.prims.integer,
            hir::ExprKind::Field(expr, ref field) => {
                let container = self.check_expression(expr)?;

                let (record, args) = match self.tcx.arenas[container] {
                    ty::Type::Record { name, ref args } => (name, args),
                    _ => {
                        self.reporter.emit(Diagnostic::from(
                            errors::TypeHasNoFields {
                                ty: container,
                                field,
                                printer: &self.printer(),
                            },
                        ));

                        return Err(TypingError);
                    }
                };

                let i = self.hir[record]
                    .fields
                    .into_iter()
                    .position(|decl| self.hir[decl].name.symbol == field.symbol)
                    .ok_or_else(|| {
                        self.reporter.emit(errors::UnexpectedField {
                            ty: &self.hir[record].name,
                            field,
                        });

                        TypingError
                    })?;

                let field = self.tcx.records[record].fields[i];

                let args = args.clone();
                let folder = Arguments::new(&args, &[]);

                field.fold_with(&mut self.tcx.arenas, &folder)
            }
            hir::ExprKind::Unary(ref op, arg) => {
                let arg = self.check_expression(arg)?;

                op.kind
                    .select_overload((arg,), &self.tcx.arenas)
                    .ok_or_else(|| {
                        self.reporter.emit(Diagnostic::from(
                            errors::NoMatchingUnaryOverload {
                                op,
                                arg,
                                printer: &self.printer(),
                            },
                        ));

                        TypingError
                    })?
            }
            hir::ExprKind::Binary(ref op, lhs, rhs) => {
                let lhs = self.check_expression(lhs)?;
                let rhs = self.check_expression(rhs)?;

                op.kind
                    .select_overload((lhs, rhs), &self.tcx.arenas)
                    .ok_or_else(|| {
                        self.reporter.emit(Diagnostic::from(
                            errors::NoMatchingBinaryOverload {
                                op,
                                lhs,
                                rhs,
                                printer: &self.printer(),
                            },
                        ));

                        TypingError
                    })?
            }
            hir::ExprKind::Call(ref call) => {
                let generics: Vec<_> = self.hir[call.generics]
                    .iter()
                    .map(|&expr| self.lower_generic_argument(expr))
                    .collect::<Result<_>>()?;

                let args: Vec<_> = self.hir[call.args]
                    .iter()
                    .map(|&expr| self.check_expression(expr))
                    .collect::<Result<_>>()?;

                let lowered: Vec<_> = self.hir[call.args]
                    .iter()
                    .zip(&args)
                    .zip(&self.hir[self.hir[call.callee].inputs])
                    .map(|((&expr, &ty), param)| match param.modifier {
                        hir::Modifier::None => {
                            Ok(if self.tcx.arenas[ty].is_real_valued() {
                                self.tcx
                                    .arenas
                                    .intern(ty::Expression::Term(expr))
                            } else {
                                Index::INVALID
                            })
                        }
                        hir::Modifier::Real => {
                            self.within("argument").lower_expression(expr)
                        }
                    })
                    .collect::<Result<_>>()?;

                let folder = Arguments::new(&generics, &lowered);

                let Signature {
                    requires,
                    inputs: ref params,
                    output,
                } = self.tcx.signatures[call.callee];

                for (i, (&param, found)) in params.iter().zip(args).enumerate()
                {
                    let expected =
                        param.fold_with(&mut self.tcx.arenas, &folder);

                    if let result @ (LBool::False | LBool::Unknown) =
                        found.equivalent(expected, &self.tcx.arenas)
                    {
                        self.reporter.emit(Diagnostic::from(
                            errors::IncompatibleTypes {
                                expr: &self.hir[self.hir[call.args][i]],
                                expected,
                                found,
                                certain: result == LBool::False,
                                printer: &self.printer(),
                            },
                        ));

                        return Err(TypingError);
                    }
                }

                if let Some(requires) = requires {
                    let prop =
                        requires.get().fold_with(&mut self.tcx.arenas, &folder);

                    if let result @ (LBool::False | LBool::Unknown) =
                        prop.entailed(self.asserts(), &self.tcx.arenas)
                    {
                        let requires =
                            self.hir[call.callee].requires.unwrap().get();

                        self.reporter.emit(errors::UnmetPrecondition {
                            callee: &call.name,
                            kind: "call",
                            requires: &self.hir[requires],
                            certain: result == LBool::False,
                        });

                        return Err(TypingError);
                    }
                }

                output.fold_with(&mut self.tcx.arenas, &folder)
            }
            hir::ExprKind::Record(ref cons) => {
                let args: Box<[_]> = self.hir[cons.generics]
                    .iter()
                    .map(|&expr| self.lower_generic_argument(expr))
                    .collect::<Result<_>>()?;

                let inits: Vec<_> = self.hir[cons.inits]
                    .iter()
                    .map(|&expr| self.check_expression(expr))
                    .collect::<Result<_>>()?;

                let folder = Arguments::new(&args, &[]);

                let Record {
                    requires,
                    ref fields,
                } = self.tcx.records[cons.record];

                for (i, (&field, found)) in fields.iter().zip(inits).enumerate()
                {
                    let expected =
                        field.fold_with(&mut self.tcx.arenas, &folder);

                    if let result @ (LBool::False | LBool::Unknown) =
                        found.equivalent(expected, &self.tcx.arenas)
                    {
                        self.reporter.emit(Diagnostic::from(
                            errors::IncompatibleTypes {
                                expr: &self.hir[self.hir[cons.inits][i]],
                                expected,
                                found,
                                certain: result == LBool::False,
                                printer: &self.printer(),
                            },
                        ));

                        return Err(TypingError);
                    }
                }

                if let Some(requires) = requires {
                    let prop =
                        requires.get().fold_with(&mut self.tcx.arenas, &folder);

                    if let result @ (LBool::False | LBool::Unknown) =
                        prop.entailed(self.asserts(), &self.tcx.arenas)
                    {
                        let requires =
                            self.hir[cons.record].requires.unwrap().get();

                        self.reporter.emit(errors::UnmetPrecondition {
                            callee: &cons.name,
                            kind: "initializer",
                            requires: &self.hir[requires],
                            certain: result == LBool::False,
                        });

                        return Err(TypingError);
                    }
                }

                self.tcx.arenas.intern(ty::Type::Record {
                    name: cons.record,
                    args,
                })
            }
        };

        Ok(ty)
    }

    fn within(
        &mut self,
        context: &'static str,
    ) -> impl ops::DerefMut<Target = Self> {
        struct Guard<'ctx, 'a, 'b> {
            ctx: &'ctx mut ItemContext<'a, 'b>,
            outer: Option<&'static str>,
        }

        impl<'a, 'b> ops::Deref for Guard<'_, 'a, 'b> {
            type Target = ItemContext<'a, 'b>;

            fn deref(&self) -> &Self::Target {
                self.ctx
            }
        }

        impl ops::DerefMut for Guard<'_, '_, '_> {
            fn deref_mut(&mut self) -> &mut Self::Target {
                self.ctx
            }
        }

        impl Drop for Guard<'_, '_, '_> {
            fn drop(&mut self) {
                self.ctx.lowering.context = self.outer;
            }
        }

        let outer = self.lowering.context.replace(context);

        Guard { ctx: self, outer }
    }

    fn printer(&self) -> Printer<'_> {
        Printer {
            hir: self.hir,
            types: &self.tcx.arenas,
            item: self.item,
        }
    }

    fn asserts(&self) -> &z3::Solver {
        self.asserts.get_or_init(z3::Solver::new)
    }
}

struct DefinitionContext<'a, 'src> {
    icx: ItemContext<'a, 'src>,
    def: Index<hir::Definition>,
}

impl DefinitionContext<'_, '_> {
    fn check_definition(&mut self) -> Result<()> {
        self.icx.check_signature(self.def)?;

        let def = &self.icx.hir[self.def];

        if let Some(expr) = def.implements {
            let expr = expr.get();

            self.icx.reporter.emit(errors::WarnNotChecked {
                what: "specifications",
                expr: &self.icx.hir[expr],
            });
        }

        if let Some(block) = def.body {
            let termination = self.check_block(block)?;

            if matches!(termination, Termination::Unit) {
                self.icx
                    .reporter
                    .emit(errors::MissingReturn { def: &def.name });

                return Err(TypingError);
            }
        }

        Ok(())
    }

    fn check_statement(
        &mut self,
        stmt: Index<hir::Statement>,
    ) -> Result<Termination> {
        match self.icx.hir[stmt].kind {
            hir::StmtKind::Let(local, expr) => {
                self.icx.tcx.env[local] = self.icx.check_expression(expr)?;

                Ok(Termination::Unit)
            }
            hir::StmtKind::Real(local, expr) => {
                self.icx.tcx.env[local] = self.icx.check_expression(expr)?;

                let lowered =
                    self.icx.within("expression").lower_expression(expr)?;

                self.icx.lowering.reals.insert(local, lowered);

                Ok(Termination::Unit)
            }
            hir::StmtKind::Assert(expr) => {
                let found = self.icx.check_expression(expr)?;
                let expected = self.icx.tcx.arenas.prims.bool;

                if found != expected {
                    self.icx.reporter.emit(Diagnostic::from(
                        errors::IncompatibleTypes {
                            expr: &self.icx.hir[expr],
                            expected,
                            found,
                            certain: true,
                            printer: &self.icx.printer(),
                        },
                    ));

                    return Err(TypingError);
                }

                let prop =
                    self.icx.within("assertion").lower_proposition(expr)?;

                if let result @ (LBool::False | LBool::Unknown) =
                    prop.entailed(self.icx.asserts(), &self.icx.tcx.arenas)
                {
                    self.icx.reporter.emit(errors::AssertionFailed {
                        assert: &self.icx.hir[stmt],
                        certain: result == LBool::False,
                    });

                    return Err(TypingError);
                }

                self.icx
                    .asserts()
                    .assert(prop.into_smt(&self.icx.tcx.arenas));

                Ok(Termination::Unit)
            }
            hir::StmtKind::Return(expr) => {
                let found = self.icx.check_expression(expr)?;
                let expected = self.icx.tcx.signatures[self.def].output;

                if let result @ (LBool::False | LBool::Unknown) =
                    found.equivalent(expected, &self.icx.tcx.arenas)
                {
                    self.icx.reporter.emit(Diagnostic::from(
                        errors::IncompatibleTypes {
                            expr: &self.icx.hir[expr],
                            expected,
                            found,
                            certain: result == LBool::False,
                            printer: &self.icx.printer(),
                        },
                    ));

                    return Err(TypingError);
                }

                Ok(Termination::Void(stmt))
            }
            hir::StmtKind::Unsafe(block) => self.check_block(block),
        }
    }

    fn check_block(
        &mut self,
        block: hir::List<hir::Statement>,
    ) -> Result<Termination> {
        let mut glb = Termination::Unit;
        let mut seen_unreachable = false;

        for &stmt in &self.icx.hir[block] {
            let termination = self.check_statement(stmt)?;

            if let Termination::Void(divergent) = glb
                && !seen_unreachable
            {
                self.icx.reporter.emit(errors::WarnUnreachable {
                    divergent: &self.icx.hir[divergent],
                    unreachable: &self.icx.hir[stmt],
                });

                seen_unreachable = true;
            }

            glb = glb.glb(termination);
        }

        Ok(glb)
    }
}

enum Termination {
    Unit,
    Void(Index<hir::Statement>),
}

impl Termination {
    fn glb(self, other: Termination) -> Termination {
        match (self, other) {
            (Self::Void(stmt), _) | (_, Self::Void(stmt)) => Self::Void(stmt),
            (Self::Unit, Self::Unit) => Self::Unit,
        }
    }
}
