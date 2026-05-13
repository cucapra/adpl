use std::cell::OnceCell;
use std::collections::HashMap;
use std::ops;

use adpl_arena::{DenseMap, Index, NonMaxIndex};
use adpl_hir as hir;
use adpl_util::{Diagnostic, Reporter};

use crate::printer::Printer;
use crate::promotion::Overloaded;
use crate::queries::{Entailed, Equivalent, IntoSmt, LBool};
use crate::substitution::Foldable;
use crate::{errors, types as ty};

pub fn check_hir(
    hir: &hir::Context,
    reporter: &mut Reporter,
) -> Option<TypingContext> {
    let mut tcx = TypingContext::with_default_env(hir);
    let mut lowering = LoweringContext::default();

    for item in hir.items.values() {
        match *item {
            hir::Item::Record(record) => {
                let mut ctx = ItemContext {
                    hir,
                    reporter,
                    tcx: &mut tcx,
                    lowering: &mut lowering,
                    asserts: OnceCell::new(),
                    generics: hir[record].params,
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
                        generics: hir[def].generics,
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
    consts: HashMap<Index<hir::Local>, Index<ty::Expression>>,
    context: Option<&'static str>,
}

impl LoweringContext {
    fn clear(&mut self) {
        self.consts.clear();
    }
}

struct ItemContext<'a, 'src> {
    hir: &'a hir::Context,
    reporter: &'a mut Reporter<'src>,
    tcx: &'a mut TypingContext,
    lowering: &'a mut LoweringContext,
    asserts: OnceCell<z3::Solver>,
    /// Generic parameters for the current item.
    generics: hir::IndexRange<hir::Local>,
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
        let expected = self.tcx.arenas.prims.integer;
        let (lowered, found) =
            self.within("generic argument").lower_expression(expr)?;

        if found == expected {
            Ok(lowered)
        } else {
            self.reporter
                .emit(Diagnostic::from(errors::IncompatibleTypes {
                    expr: &self.hir[expr],
                    expected,
                    found,
                    certain: true,
                    printer: &self.printer(),
                }));

            Err(TypingError)
        }
    }

    fn lower_expression(
        &mut self,
        expr: Index<hir::Expression>,
    ) -> Result<(Index<ty::Expression>, Index<ty::Type>)> {
        match self.hir[expr].kind {
            hir::ExprKind::Id(local) => match self.hir[local].kind {
                hir::LocalKind::Const(_) => {
                    Ok((self.lowering.consts[&local], self.tcx.env[local]))
                }
                hir::LocalKind::Let(_) | hir::LocalKind::Param(_) => {
                    self.reporter.emit(errors::ExpectedConst {
                        expr: &self.hir[expr],
                    });

                    Err(TypingError)
                }
                hir::LocalKind::GenericParam(i) => {
                    let lowered =
                        self.tcx.arenas.intern(ty::Expression::Param(i));

                    Ok((lowered, self.tcx.arenas.prims.integer))
                }
            },
            hir::ExprKind::Lit(ref literal) => {
                let lowered = self
                    .tcx
                    .arenas
                    .intern(ty::Expression::Const(literal.value));

                Ok((lowered, self.tcx.arenas.prims.integer))
            }
            hir::ExprKind::Field(expr, ref field) => {
                let (_, ty) = self.lower_expression(expr)?;

                match self.tcx.arenas[ty] {
                    ty::Type::Record { .. } => unreachable!(),
                    _ => {
                        self.reporter.emit(Diagnostic::from(
                            errors::TypeHasNoFields {
                                ty,
                                field,
                                printer: &self.printer(),
                            },
                        ));

                        Err(TypingError)
                    }
                }
            }
            hir::ExprKind::Unary(ref op, arg) => {
                let (arg, arg_ty) = self.lower_expression(arg)?;

                let overload = op
                    .kind
                    .select_overload(arg_ty, &self.tcx.arenas)
                    .ok_or_else(|| {
                        self.reporter.emit(Diagnostic::from(
                            errors::NoMatchingUnaryOverload {
                                op,
                                arg: arg_ty,
                                printer: &self.printer(),
                            },
                        ));

                        TypingError
                    })?;

                match op.kind {
                    hir::UnaryKind::Neg => {
                        let lowered =
                            self.tcx.arenas.intern(ty::Expression::Neg(arg));

                        Ok((lowered, overload))
                    }
                    hir::UnaryKind::Not => unreachable!(),
                }
            }
            hir::ExprKind::Binary(ref op, lhs, rhs) => {
                let (lhs, lhs_ty) = self.lower_expression(lhs)?;
                let (rhs, rhs_ty) = self.lower_expression(rhs)?;

                let overload = op
                    .kind
                    .select_binary_overload(lhs_ty, rhs_ty, &self.tcx.arenas)
                    .ok_or_else(|| {
                        self.reporter.emit(Diagnostic::from(
                            errors::NoMatchingBinaryOverload {
                                op,
                                lhs: lhs_ty,
                                rhs: rhs_ty,
                                printer: &self.printer(),
                            },
                        ));

                        TypingError
                    })?;

                let Ok(op) = op.kind.try_into() else {
                    self.reporter.emit(errors::NoDenotation {
                        op: errors::OpKind::Binary(op),
                        context: self.lowering.context.unwrap(),
                    });

                    return Err(TypingError);
                };

                let lowered = self
                    .tcx
                    .arenas
                    .intern(ty::Expression::Binary(op, lhs, rhs));

                Ok((lowered, overload))
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

    fn lower_proposition(
        &mut self,
        expr: Index<hir::Expression>,
    ) -> Result<Index<ty::Proposition>> {
        match self.hir[expr].kind {
            hir::ExprKind::Id(local) => match self.hir[local].kind {
                hir::LocalKind::Let(_) => unreachable!(),
                hir::LocalKind::Const(_) => unreachable!(),
                hir::LocalKind::Param(_) => {
                    self.reporter.emit(errors::ExpectedConst {
                        expr: &self.hir[expr],
                    });

                    Err(TypingError)
                }
                hir::LocalKind::GenericParam(_) => {
                    self.reporter.emit(Diagnostic::from(
                        errors::IncompatibleTypes {
                            expr: &self.hir[expr],
                            expected: self.tcx.arenas.prims.bool,
                            found: self.tcx.arenas.prims.integer,
                            certain: true,
                            printer: &self.printer(),
                        },
                    ));

                    Err(TypingError)
                }
            },
            hir::ExprKind::Lit(_) => {
                self.reporter.emit(Diagnostic::from(
                    errors::IncompatibleTypes {
                        expr: &self.hir[expr],
                        expected: self.tcx.arenas.prims.bool,
                        found: self.tcx.arenas.prims.integer,
                        certain: true,
                        printer: &self.printer(),
                    },
                ));

                Err(TypingError)
            }
            hir::ExprKind::Field(..) => {
                Err(self.lower_expression(expr).unwrap_err())
            }
            hir::ExprKind::Unary(ref op, arg) => match op.kind {
                hir::UnaryKind::Neg => {
                    let (_, found) = self.lower_expression(expr)?;

                    self.reporter.emit(Diagnostic::from(
                        errors::IncompatibleTypes {
                            expr: &self.hir[expr],
                            expected: self.tcx.arenas.prims.bool,
                            found,
                            certain: true,
                            printer: &self.printer(),
                        },
                    ));

                    Err(TypingError)
                }
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
                    let (lhs, lhs_ty) = self.lower_expression(lhs)?;
                    let (rhs, rhs_ty) = self.lower_expression(rhs)?;

                    let found = kind
                        .select_binary_overload(
                            lhs_ty,
                            rhs_ty,
                            &self.tcx.arenas,
                        )
                        .ok_or_else(|| {
                            self.reporter.emit(Diagnostic::from(
                                errors::NoMatchingBinaryOverload {
                                    op,
                                    lhs: lhs_ty,
                                    rhs: rhs_ty,
                                    printer: &self.printer(),
                                },
                            ));

                            TypingError
                        })?;

                    let Ok(op) = kind.try_into() else {
                        self.reporter.emit(Diagnostic::from(
                            errors::IncompatibleTypes {
                                expr: &self.hir[expr],
                                expected: self.tcx.arenas.prims.bool,
                                found,
                                certain: true,
                                printer: &self.printer(),
                            },
                        ));

                        return Err(TypingError);
                    };

                    Ok(self
                        .tcx
                        .arenas
                        .intern(ty::Proposition::Relation(op, lhs, rhs)))
                }
            },
            hir::ExprKind::Call(_) => {
                Err(self.lower_expression(expr).unwrap_err())
            }
            hir::ExprKind::Record(_) => {
                Err(self.lower_expression(expr).unwrap_err())
            }
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
                self.within("precondition")
                    .lower_proposition(expr.get())
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
            self.tcx.env[param.local] = self.lower_type(param.ty)?;
        }

        let inputs = self.hir[def.inputs]
            .iter()
            .map(|param| self.tcx.env[param.local])
            .collect();

        let output = self.lower_type(def.output)?;

        let requires = def
            .requires
            .map(|expr| {
                let prop = self
                    .within("precondition")
                    .lower_proposition(expr.get())?;

                self.asserts().assert(prop.into_smt(&self.tcx.arenas));

                Ok(prop.try_into().unwrap())
            })
            .transpose()?;

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

                field.fold_with(&mut self.tcx.arenas, args.as_ref())
            }
            hir::ExprKind::Unary(ref op, arg) => {
                let arg = self.check_expression(arg)?;

                op.kind.select_overload(arg, &self.tcx.arenas).ok_or_else(
                    || {
                        self.reporter.emit(Diagnostic::from(
                            errors::NoMatchingUnaryOverload {
                                op,
                                arg,
                                printer: &self.printer(),
                            },
                        ));

                        TypingError
                    },
                )?
            }
            hir::ExprKind::Binary(ref op, lhs, rhs) => {
                let lhs = self.check_expression(lhs)?;
                let rhs = self.check_expression(rhs)?;

                op.kind
                    .select_binary_overload(lhs, rhs, &self.tcx.arenas)
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

                let Signature {
                    requires,
                    inputs: ref params,
                    output,
                } = self.tcx.signatures[call.callee];

                for (i, (&param, found)) in params.iter().zip(args).enumerate()
                {
                    let expected = param
                        .fold_with(&mut self.tcx.arenas, generics.as_slice());

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
                    let prop = requires
                        .get()
                        .fold_with(&mut self.tcx.arenas, generics.as_slice());

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

                output.fold_with(&mut self.tcx.arenas, generics.as_slice())
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

                let Record {
                    requires,
                    ref fields,
                } = self.tcx.records[cons.record];

                for (i, (&field, found)) in fields.iter().zip(inits).enumerate()
                {
                    let expected =
                        field.fold_with(&mut self.tcx.arenas, args.as_ref());

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
                    let prop = requires
                        .get()
                        .fold_with(&mut self.tcx.arenas, args.as_ref());

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
            params: self.generics,
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
            hir::StmtKind::Assign(local, expr) => {
                self.icx.tcx.env[local] = self.icx.check_expression(expr)?;

                Ok(Termination::Unit)
            }
            hir::StmtKind::Const(local, expr) => {
                let (lowered, ty) =
                    self.icx.within("constant").lower_expression(expr)?;

                self.icx.tcx.env[local] = ty;
                self.icx.lowering.consts.insert(local, lowered);

                Ok(Termination::Unit)
            }
            hir::StmtKind::Return(expr) => {
                let expected = self.icx.tcx.signatures[self.def].output;
                let found = self.icx.check_expression(expr)?;

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
