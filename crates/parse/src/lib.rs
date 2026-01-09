use std::ops::Range;

use chumsky::error::Cheap;
use chumsky::extra;
use chumsky::input::{Input as _, MapExtra, Stream, ValueInput};
use chumsky::pratt::{infix, left, postfix, prefix, right};
use chumsky::primitive::{choice, group, just};
use chumsky::recursive::recursive;
use chumsky::{IterParser as _, Parser, select};

use adpl_ast as ast;
use adpl_lex::{Lexer, Token};

pub type Span = Range<usize>;
pub type Error = Cheap<Span>;

enum AtomTail {
    Call(Vec<ast::Expression>),
    Record(Vec<ast::Assignment>),
}

fn parser<'tk, 'src, I>() -> impl Parser<'tk, I, ast::File, extra::Err<Error>>
where
    I: ValueInput<'tk, Token = Token<'src>, Span = Span>,
    'src: 'tk,
{
    let id = select! {
        Token::Ident(symbol) = e => ast::Id {
            symbol: ast::Symbol::from(symbol),
            span: ast::Span::from(e.span()),
        },
    };

    let lit = select! {
        Token::Literal(text) => text,
    }
    .try_map(|text, span| {
        text.parse()
            .map(|value| ast::Literal { value })
            .map_err(|_| Error::new(span))
    });

    let expr = recursive(|expr| {
        let generics = expr
            .clone()
            .separated_by(just(Token::Comma))
            .allow_trailing()
            .collect()
            .delimited_by(just(Token::OpenBracket), just(Token::CloseBracket));

        let args = expr
            .clone()
            .separated_by(just(Token::Comma))
            .allow_trailing()
            .collect()
            .delimited_by(just(Token::OpenParen), just(Token::CloseParen));

        let fields = id
            .then_ignore(just(Token::Eq))
            .then(expr.clone())
            .map(|(lhs, rhs)| ast::Assignment { lhs, rhs })
            .separated_by(just(Token::Comma))
            .allow_trailing()
            .collect()
            .delimited_by(just(Token::OpenBrace), just(Token::CloseBrace));

        let call_like = id
            .then(generics.or_not().map(|args| args.unwrap_or_else(Vec::new)))
            .then(choice((
                args.map(AtomTail::Call),
                fields.map(AtomTail::Record),
            )))
            .map(|((name, generics), tail)| match tail {
                AtomTail::Call(args) => {
                    ast::ExprKind::Call(Box::new(ast::Call {
                        name,
                        generics,
                        args,
                    }))
                }
                AtomTail::Record(fields) => {
                    ast::ExprKind::Record(Box::new(ast::Constructor {
                        name,
                        generics,
                        fields,
                    }))
                }
            });

        let atom = choice((
            call_like,
            id.map(ast::ExprKind::Id),
            lit.map(ast::ExprKind::Lit),
            expr.map(|expr| expr.kind)
                .delimited_by(just(Token::OpenParen), just(Token::CloseParen)),
        ))
        .map_with(|kind, e| ast::Expression {
            kind,
            span: ast::Span::from(e.span()),
        });

        let map_binary =
            |kind, e: &mut MapExtra<'tk, '_, I, _>| ast::BinaryOp {
                kind,
                span: ast::Span::from(e.span()),
            };

        let fold_binary =
            |lhs, op, rhs, e: &mut MapExtra<'tk, '_, I, _>| ast::Expression {
                kind: ast::ExprKind::Binary(op, Box::new(lhs), Box::new(rhs)),
                span: ast::Span::from(e.span()),
            };

        atom.pratt((
            postfix(7, just(Token::Dot).ignore_then(id), |lhs, id, e| {
                ast::Expression {
                    kind: ast::ExprKind::Field(Box::new(lhs), id),
                    span: ast::Span::from(e.span()),
                }
            }),
            infix(
                right(6),
                just(Token::Caret)
                    .to(ast::BinaryKind::Pow)
                    .map_with(map_binary),
                fold_binary,
            ),
            prefix(
                5,
                choice((
                    just(Token::Minus).to(ast::UnaryKind::Neg),
                    just(Token::Bang).to(ast::UnaryKind::Not),
                ))
                .map_with(|kind, e| ast::UnaryOp {
                    kind,
                    span: ast::Span::from(e.span()),
                }),
                |op, rhs, e| ast::Expression {
                    kind: ast::ExprKind::Unary(op, Box::new(rhs)),
                    span: ast::Span::from(e.span()),
                },
            ),
            infix(
                left(4),
                choice((
                    just(Token::Star).to(ast::BinaryKind::Mul),
                    just(Token::Slash).to(ast::BinaryKind::Div),
                ))
                .map_with(map_binary),
                fold_binary,
            ),
            infix(
                left(3),
                choice((
                    just(Token::Plus).to(ast::BinaryKind::Add),
                    just(Token::Minus).to(ast::BinaryKind::Sub),
                ))
                .map_with(map_binary),
                fold_binary,
            ),
            infix(
                left(2),
                choice((
                    just(Token::Shl).to(ast::BinaryKind::Shl),
                    just(Token::Shr).to(ast::BinaryKind::Shr),
                ))
                .map_with(map_binary),
                fold_binary,
            ),
            infix(
                left(1),
                choice((
                    just(Token::Gt).to(ast::BinaryKind::Gt),
                    just(Token::Ge).to(ast::BinaryKind::Ge),
                    just(Token::Lt).to(ast::BinaryKind::Lt),
                    just(Token::Le).to(ast::BinaryKind::Le),
                ))
                .map_with(map_binary),
                fold_binary,
            ),
            infix(
                left(0),
                choice((
                    just(Token::Eq).to(ast::BinaryKind::Eq),
                    just(Token::Ne).to(ast::BinaryKind::Ne),
                ))
                .map_with(map_binary),
                fold_binary,
            ),
        ))
    });

    let block = recursive(|block| {
        let statement = choice((
            id.then_ignore(just(Token::Eq))
                .then(expr.clone())
                .then_ignore(just(Token::Semicolon))
                .map(|(lhs, rhs)| {
                    ast::StmtKind::Assign(ast::Assignment { lhs, rhs })
                }),
            just(Token::Return)
                .ignore_then(expr.clone())
                .then_ignore(just(Token::Semicolon))
                .map(ast::StmtKind::Return),
            just(Token::Unsafe)
                .ignore_then(block)
                .map(ast::StmtKind::Unsafe),
        ))
        .map(|kind| ast::Statement { kind });

        statement
            .repeated()
            .collect()
            .delimited_by(just(Token::OpenBrace), just(Token::CloseBrace))
            .map(ast::Block)
    });

    let args = expr
        .clone()
        .separated_by(just(Token::Comma))
        .allow_trailing()
        .collect()
        .delimited_by(just(Token::OpenBracket), just(Token::CloseBracket));

    let ty = id
        .then(args.or_not())
        .map_with(|(name, args), e| ast::Type {
            name,
            args: args.unwrap_or_else(Vec::new),
            span: ast::Span::from(e.span()),
        })
        .boxed();

    let generics = id
        .separated_by(just(Token::Comma))
        .allow_trailing()
        .collect()
        .delimited_by(just(Token::OpenBracket), just(Token::CloseBracket));

    let fields = id
        .then_ignore(just(Token::Colon))
        .then(ty.clone())
        .map_with(|(name, ty), e| ast::Field {
            name,
            ty,
            span: ast::Span::from(e.span()),
        })
        .separated_by(just(Token::Comma))
        .allow_trailing()
        .collect()
        .delimited_by(just(Token::OpenBrace), just(Token::CloseBrace));

    let record = just(Token::Struct)
        .ignore_then(id)
        .then(generics.or_not())
        .then(fields)
        .map(|((name, params), fields)| {
            ast::ItemKind::Record(ast::Record {
                name,
                params: params.unwrap_or_else(Vec::new),
                fields,
            })
        });

    let safety = just(Token::Unsafe)
        .to(ast::Safety::Unsafe)
        .or_not()
        .map(|safety| safety.unwrap_or(ast::Safety::Safe));

    let params = id
        .then_ignore(just(Token::Colon))
        .then(ty.clone())
        .map_with(|(name, ty), e| ast::Parameter {
            name,
            ty,
            span: ast::Span::from(e.span()),
        })
        .separated_by(just(Token::Comma))
        .allow_trailing()
        .collect()
        .delimited_by(just(Token::OpenParen), just(Token::CloseParen));

    let signature = params
        .then_ignore(just(Token::Arrow))
        .then(ty)
        .map(|(inputs, output)| Box::new(ast::Signature { inputs, output }));

    let requires = just(Token::Where).ignore_then(expr.clone()).map(Box::new);
    let implements = just(Token::Implements).ignore_then(expr).map(Box::new);

    let definition = group((
        safety,
        just(Token::Def).ignore_then(id),
        generics.or_not(),
        signature,
        requires.or_not(),
        implements.or_not(),
        block.map(Some).or(just(Token::Semicolon).map(|_| None)),
    ))
    .map(
        |(safety, name, generics, sig, requires, implements, body)| {
            ast::ItemKind::Def(ast::Definition {
                safety,
                name,
                generics: generics.unwrap_or_else(Vec::new),
                requires,
                implements,
                sig,
                body,
            })
        },
    );

    choice((record, definition))
        .map(|kind| ast::Item { kind })
        .repeated()
        .collect()
        .map(|items| ast::File { items })
}

pub fn parse(src: &str) -> Result<ast::File, Vec<Error>> {
    let lexer = Lexer::new(src)
        .spanned()
        .map(|(tk, span)| (tk.unwrap_or(Token::Error), span));

    let eoi = src.len()..src.len();
    let stream = Stream::from_iter(lexer).map(eoi, |tk| tk);

    parser().parse(stream).into_result()
}
