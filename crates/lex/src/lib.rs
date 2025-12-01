use logos::Logos;

#[derive(Logos, Clone, Copy, PartialEq)]
#[logos(skip r"[ \t\v\r\n\f]+")]
#[logos(skip r"//[^\n]*\n")]
#[logos(skip r"/\*[^*]*\*+([^/*][^*]*\*+)*/")]
pub enum Token<'src> {
    Error,

    #[regex("[a-zA-Z_][0-9a-zA-Z_]*")]
    Ident(&'src str),

    #[regex("[0-9]+")]
    Literal(&'src str),

    #[token("def")]
    Def,
    #[token("else")]
    Else,
    #[token("if")]
    If,
    #[token("implements")]
    Implements,
    #[token("return")]
    Return,
    #[token("struct")]
    Struct,
    #[token("unsafe")]
    Unsafe,
    #[token("where")]
    Where,

    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("^")]
    Caret,
    #[token("<<")]
    Shl,
    #[token(">>")]
    Shr,

    #[token("!")]
    Bang,
    #[token("=")]
    Eq,
    #[token("!=")]
    Ne,
    #[token(">")]
    Gt,
    #[token(">=")]
    Ge,
    #[token("<")]
    Lt,
    #[token("<=")]
    Le,

    #[token("->")]
    Arrow,
    #[token(":")]
    Colon,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    #[token(";")]
    Semicolon,

    #[token("(")]
    OpenParen,
    #[token(")")]
    CloseParen,
    #[token("{")]
    OpenBrace,
    #[token("}")]
    CloseBrace,
    #[token("[")]
    OpenBracket,
    #[token("]")]
    CloseBracket,
}

pub type Lexer<'src> = logos::Lexer<'src, Token<'src>>;
