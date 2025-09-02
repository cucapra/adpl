use logos::Logos;

#[derive(Logos)]
#[logos(skip r"[ \t\v\r\n\f]+")]
#[logos(skip r"//[^\n]*\n")]
#[logos(skip r"/\*[^*]*\*+([^/*][^*]*\*+)*/")]
pub enum Token {
    #[regex("[a-zA-Z_][0-9a-zA-Z_]*")]
    Ident,

    #[regex("[0-9]+")]
    Literal,

    #[token("def")]
    Def,
    #[token("else")]
    Else,
    #[token("if")]
    If,
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
    #[token("_", priority = 3)]
    Underscore,

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
