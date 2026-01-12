use std::collections::HashMap;
use std::iter;

use adpl_ast as ast;
use adpl_hir as hir;
use adpl_util::{Reporter, with_sufficient_stack};

use crate::errors;

pub fn lower_ast(
    file: &ast::File,
    reporter: &mut Reporter,
) -> Option<hir::Context> {
    let mut ctx = hir::Context::new();

    let mut lowering = LoweringContext {
        ctx: &mut ctx,
        reporter,
        globals: HashMap::new(),
        scopes: Vec::new(),
    };

    lowering.lower_file(file).ok()?;

    Some(ctx)
}

#[derive(Debug)]
struct LoweringError;

type Result<T> = std::result::Result<T, LoweringError>;

struct LoweringContext<'a, 'src> {
    ctx: &'a mut hir::Context,
    reporter: &'a mut Reporter<'src>,
    globals: HashMap<ast::Symbol, Global>,
    scopes: Vec<HashMap<ast::Symbol, hir::Index<hir::Local>>>,
}

impl LoweringContext<'_, '_> {
    fn lower_file(&mut self, file: &ast::File) -> Result<()> {
        for item in &file.items {
            match &item.kind {
                ast::ItemKind::Record(record) => {
                    self.lower_record(record)?;
                }
                ast::ItemKind::Def(def) => {
                    self.lower_definition(def)?;
                }
            }
        }

        Ok(())
    }

    fn lower_record(
        &mut self,
        record: &ast::Record,
    ) -> Result<hir::Index<hir::Record>> {
        let params =
            self.ctx.locals.extend(record.params.iter().enumerate().map(
                |(i, param)| hir::Local {
                    kind: hir::LocalKind::GenericParam(i.try_into().unwrap()),
                    name: *param,
                },
            ));

        let mut scope = HashMap::with_capacity(record.params.len());

        for (param, local) in iter::zip(&record.params, params) {
            if scope.insert(param.symbol, local).is_some() {
                self.reporter
                    .emit(errors::ReusedParameter { second: param });

                return Err(LoweringError);
            }
        }

        self.scopes.push(scope);
        let start = self.ctx.fields.next_index();

        for (i, field) in record.fields.iter().enumerate() {
            self.lower_field(field)?;

            for prev in &record.fields[..i] {
                if prev.name.symbol == field.name.symbol {
                    self.reporter.emit(errors::RedeclaredField {
                        first: &prev.name,
                        second: &field.name,
                    });

                    return Err(LoweringError);
                }
            }
        }

        let end = self.ctx.fields.next_index();
        self.scopes.pop();

        let index = self.ctx.add(hir::Record {
            name: record.name,
            params,
            fields: hir::IndexRange { start, end },
        });

        self.add_global(&record.name, Global::Record(index))?;

        Ok(index)
    }

    fn lower_field(
        &mut self,
        field: &ast::Field,
    ) -> Result<hir::Index<hir::Field>> {
        let ty = self.lower_type(&field.ty)?;

        Ok(self.ctx.add(hir::Field {
            name: field.name,
            ty,
            span: field.span,
        }))
    }

    fn lower_type(&mut self, ty: &ast::Type) -> Result<hir::Index<hir::Type>> {
        let decl = self
            .globals
            .get(&ty.name.symbol)
            .ok_or_else(|| {
                self.reporter.emit(errors::KindNotFound {
                    name: &ty.name,
                    kind: "type",
                });

                LoweringError
            })
            .and_then(|&name| match name {
                Global::Record(record) => Ok(record),
                Global::Def(_) => {
                    self.reporter.emit(errors::UnexpectedKind {
                        name: &ty.name,
                        expected: "type",
                        found: "function",
                        label: "not a type",
                    });

                    Err(LoweringError)
                }
            })?;

        let args = self.ctx.lists.extend_zeroed(ty.args.len());

        for (i, arg) in ty.args.iter().enumerate() {
            self.ctx[args][i] = self.lower_expression(arg)?;
        }

        let declared_param_count = self.ctx[decl].params.len();
        let supplied_param_count = ty.args.len();

        if declared_param_count != supplied_param_count {
            self.reporter.emit(errors::ArityMismatch {
                callee: &ty.name,
                expected: declared_param_count,
                found: supplied_param_count,
                what: "generic argument",
            });

            return Err(LoweringError);
        }

        Ok(self.ctx.add(hir::Type {
            name: ty.name,
            decl,
            args,
            span: ty.span,
        }))
    }

    fn lower_definition(
        &mut self,
        def: &ast::Definition,
    ) -> Result<hir::Index<hir::Definition>> {
        let generics =
            self.ctx.locals.extend(def.generics.iter().enumerate().map(
                |(i, param)| hir::Local {
                    kind: hir::LocalKind::GenericParam(i.try_into().unwrap()),
                    name: *param,
                },
            ));

        let params =
            self.ctx
                .locals
                .extend(def.sig.inputs.iter().enumerate().map(|(i, param)| {
                    hir::Local {
                        kind: hir::LocalKind::Param(i.try_into().unwrap()),
                        name: param.name,
                    }
                }));

        let mut scope =
            HashMap::with_capacity(def.generics.len() + def.sig.inputs.len());

        for (param, local) in iter::zip(&def.generics, generics) {
            if scope.insert(param.symbol, local).is_some() {
                self.reporter
                    .emit(errors::ReusedParameter { second: param });

                return Err(LoweringError);
            }
        }

        self.scopes.push(scope);

        let start = self.ctx.params.next_index();

        for (param, local) in iter::zip(&def.sig.inputs, params) {
            self.lower_parameter(param, local)?;

            if let Some(prev) = self
                .scopes
                .last_mut()
                .unwrap()
                .insert(param.name.symbol, local)
            {
                let prev = &self.ctx[prev];

                match prev.kind {
                    hir::LocalKind::GenericParam(_) => {
                        self.reporter.emit(errors::ShadowedGeneric {
                            first: &prev.name,
                            second: &param.name,
                        });
                    }
                    hir::LocalKind::Param(_) => {
                        self.reporter.emit(errors::ReusedParameter {
                            second: &param.name,
                        });
                    }
                    _ => unreachable!(),
                }

                return Err(LoweringError);
            }
        }

        let end = self.ctx.params.next_index();

        let output = self.lower_type(&def.sig.output)?;

        let requires = def
            .requires
            .as_deref()
            .map(|requires| {
                self.lower_expression(requires)
                    .map(|index| index.try_into().unwrap())
            })
            .transpose()?;

        let implements = def
            .implements
            .as_deref()
            .map(|implements| {
                self.lower_expression(implements)
                    .map(|index| index.try_into().unwrap())
            })
            .transpose()?;

        let body = def
            .body
            .as_ref()
            .map(|block| self.lower_block(block))
            .transpose()?;

        self.scopes.pop();

        let index = self.ctx.add(hir::Definition {
            safety: def.safety,
            name: def.name,
            generics,
            requires,
            implements,
            inputs: hir::IndexRange { start, end },
            output,
            body,
        });

        self.add_global(&def.name, Global::Def(index))?;

        Ok(index)
    }

    fn lower_parameter(
        &mut self,
        param: &ast::Parameter,
        local: hir::Index<hir::Local>,
    ) -> Result<hir::Index<hir::Parameter>> {
        let ty = self.lower_type(&param.ty)?;

        Ok(self.ctx.add(hir::Parameter {
            local,
            ty,
            span: param.span,
        }))
    }

    fn lower_statement(
        &mut self,
        stmt: &ast::Statement,
    ) -> Result<hir::Index<hir::Statement>> {
        let kind = match &stmt.kind {
            ast::StmtKind::Assign(assn) => {
                let expr = self.lower_expression(&assn.rhs)?;

                let local = self.ctx.add(hir::Local {
                    kind: hir::LocalKind::Let(expr),
                    name: assn.lhs,
                });

                self.scopes
                    .last_mut()
                    .unwrap()
                    .insert(assn.lhs.symbol, local);

                hir::StmtKind::Assign(local, expr)
            }
            ast::StmtKind::Return(expr) => {
                hir::StmtKind::Return(self.lower_expression(expr)?)
            }
            ast::StmtKind::Unsafe(block) => {
                hir::StmtKind::Unsafe(self.lower_block(block)?)
            }
        };

        Ok(self.ctx.add(hir::Statement { kind }))
    }

    fn lower_block(
        &mut self,
        block: &ast::Block,
    ) -> Result<hir::List<hir::Statement>> {
        let list = self.ctx.lists.extend_zeroed(block.0.len());

        for (i, stmt) in block.0.iter().enumerate() {
            self.ctx[list][i] = self.lower_statement(stmt)?;
        }

        Ok(list)
    }

    fn lower_expression(
        &mut self,
        expr: &ast::Expression,
    ) -> Result<hir::Index<hir::Expression>> {
        with_sufficient_stack(|| {
            let kind = match &expr.kind {
                ast::ExprKind::Id(name) => {
                    let Some(local) = self.find_name(name.symbol) else {
                        if let Some(global) = self.globals.get(&name.symbol) {
                            self.reporter.emit(errors::UnexpectedItem {
                                name,
                                kind: global.kind(),
                            });
                        } else {
                            self.reporter.emit(errors::UndefinedName { name });
                        }

                        return Err(LoweringError);
                    };

                    hir::ExprKind::Id(local)
                }
                ast::ExprKind::Lit(literal) => {
                    hir::ExprKind::Lit(literal.clone())
                }
                ast::ExprKind::Field(expr, name) => {
                    hir::ExprKind::Field(self.lower_expression(expr)?, *name)
                }
                ast::ExprKind::Unary(op, expr) => {
                    hir::ExprKind::Unary(*op, self.lower_expression(expr)?)
                }
                ast::ExprKind::Binary(op, lhs, rhs) => hir::ExprKind::Binary(
                    *op,
                    self.lower_expression(lhs)?,
                    self.lower_expression(rhs)?,
                ),
                ast::ExprKind::Call(call) => {
                    hir::ExprKind::Call(self.lower_call(call)?)
                }
                ast::ExprKind::Record(cons) => {
                    hir::ExprKind::Record(self.lower_constructor(cons)?)
                }
            };

            Ok(self.ctx.add(hir::Expression {
                kind,
                span: expr.span,
            }))
        })
    }

    fn lower_call(&mut self, call: &ast::Call) -> Result<hir::Call> {
        let callee = self
            .globals
            .get(&call.name.symbol)
            .ok_or_else(|| {
                self.reporter.emit(errors::KindNotFound {
                    name: &call.name,
                    kind: "function",
                });

                LoweringError
            })
            .and_then(|&name| match name {
                Global::Record(_) => {
                    self.reporter.emit(errors::UnexpectedKind {
                        name: &call.name,
                        expected: "function",
                        found: "struct",
                        label: "struct not callable",
                    });

                    Err(LoweringError)
                }
                Global::Def(def) => Ok(def),
            })?;

        let generics = self.ctx.lists.extend_zeroed(call.generics.len());

        for (i, expr) in call.generics.iter().enumerate() {
            self.ctx[generics][i] = self.lower_expression(expr)?;
        }

        let declared_generic_count = self.ctx[callee].generics.len();
        let supplied_generic_count = call.generics.len();

        if declared_generic_count != supplied_generic_count {
            self.reporter.emit(errors::ArityMismatch {
                callee: &call.name,
                expected: declared_generic_count,
                found: supplied_generic_count,
                what: "generic argument",
            });

            return Err(LoweringError);
        }

        let args = self.ctx.lists.extend_zeroed(call.args.len());

        for (i, expr) in call.args.iter().enumerate() {
            self.ctx[args][i] = self.lower_expression(expr)?;
        }

        let declared_arg_count = self.ctx[callee].inputs.len();
        let supplied_arg_count = call.args.len();

        if declared_arg_count != supplied_arg_count {
            self.reporter.emit(errors::ArityMismatch {
                callee: &call.name,
                expected: declared_arg_count,
                found: supplied_arg_count,
                what: "argument",
            });

            return Err(LoweringError);
        }

        Ok(hir::Call {
            name: call.name,
            callee,
            generics,
            args,
        })
    }

    fn lower_constructor(
        &mut self,
        cons: &ast::Constructor,
    ) -> Result<hir::Constructor> {
        let record = self
            .globals
            .get(&cons.name.symbol)
            .ok_or_else(|| {
                self.reporter.emit(errors::KindNotFound {
                    name: &cons.name,
                    kind: "struct",
                });

                LoweringError
            })
            .and_then(|&name| match name {
                Global::Record(record) => Ok(record),
                Global::Def(_) => {
                    self.reporter.emit(errors::UnexpectedKind {
                        name: &cons.name,
                        expected: "struct",
                        found: "function",
                        label: "not a struct type",
                    });

                    Err(LoweringError)
                }
            })?;

        let generics = self.ctx.lists.extend_zeroed(cons.generics.len());

        for (i, expr) in cons.generics.iter().enumerate() {
            self.ctx[generics][i] = self.lower_expression(expr)?;
        }

        let declared_param_count = self.ctx[record].params.len();
        let supplied_param_count = cons.generics.len();

        if declared_param_count != supplied_param_count {
            self.reporter.emit(errors::ArityMismatch {
                callee: &cons.name,
                expected: declared_param_count,
                found: supplied_param_count,
                what: "generic argument",
            });

            return Err(LoweringError);
        }

        let fields = self.ctx[record].fields;
        let inits = self.ctx.lists.extend_invalid(fields.len());

        for init in &cons.fields {
            let i = fields
                .into_iter()
                .position(|field| {
                    self.ctx[field].name.symbol == init.lhs.symbol
                })
                .ok_or_else(|| {
                    self.reporter.emit(errors::UnexpectedField {
                        ty: &cons.name,
                        field: &init.lhs,
                    });

                    LoweringError
                })?;

            if self.ctx[inits][i] != hir::Index::INVALID {
                self.reporter
                    .emit(errors::DuplicateField { second: &init.lhs });

                return Err(LoweringError);
            }

            self.ctx[inits][i] = self.lower_expression(&init.rhs)?;
        }

        for (field, &init) in iter::zip(fields, &self.ctx[inits]) {
            if init == hir::Index::INVALID {
                self.reporter.emit(errors::MissingField {
                    ty: &cons.name,
                    field: &self.ctx[field].name,
                });

                return Err(LoweringError);
            }
        }

        Ok(hir::Constructor {
            name: cons.name,
            record,
            generics,
            inits,
        })
    }

    fn add_global(&mut self, name: &ast::Id, global: Global) -> Result<()> {
        if let Some(prev) = self.globals.insert(name.symbol, global) {
            self.reporter.emit(errors::RedefinedName {
                first: prev.name(self.ctx),
                second: name,
            });

            Err(LoweringError)
        } else {
            Ok(())
        }
    }

    fn find_name(&self, symbol: ast::Symbol) -> Option<hir::Index<hir::Local>> {
        for scope in self.scopes.iter().rev() {
            if let Some(&local) = scope.get(&symbol) {
                return Some(local);
            }
        }

        None
    }
}

#[derive(Clone, Copy)]
enum Global {
    Record(hir::Index<hir::Record>),
    Def(hir::Index<hir::Definition>),
}

impl Global {
    fn name(self, ctx: &hir::Context) -> &hir::Id {
        match self {
            Global::Record(record) => &ctx[record].name,
            Global::Def(def) => &ctx[def].name,
        }
    }

    fn kind(self) -> &'static str {
        match self {
            Global::Record(_) => "struct",
            Global::Def(_) => "function",
        }
    }
}
