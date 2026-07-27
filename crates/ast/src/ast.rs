use std::ops::Range;

use symbol_table::GlobalSymbol;

pub type Symbol = GlobalSymbol;

#[derive(Debug)]
pub struct File {
    pub items: Vec<Item>,
}

#[derive(Debug)]
pub struct Item {
    pub kind: ItemKind,
}

#[derive(Debug)]
pub enum ItemKind {
    Record(Record),
    Def(Definition),
}

#[derive(Debug)]
pub struct Record {
    pub name: Id,
    pub params: Vec<Id>,
    pub requires: Option<Box<Expression>>,
    pub fields: Vec<Field>,
}

#[derive(Debug)]
pub struct Field {
    pub name: Id,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug)]
pub struct Type {
    pub name: Id,
    pub args: Vec<Expression>,
    pub span: Span,
}

#[derive(Debug)]
pub struct Definition {
    pub safety: Safety,
    pub name: Id,
    pub generics: Vec<Id>,
    pub requires: Option<Box<Expression>>,
    pub implements: Option<Box<Expression>>,
    pub sig: Box<Signature>,
    pub body: Option<Block>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Safety {
    Unsafe,
    Safe,
}

#[derive(Debug)]
pub struct Signature {
    pub inputs: Vec<Parameter>,
    pub output: Type,
}

#[derive(Debug)]
pub struct Parameter {
    pub modifier: Modifier,
    pub name: Id,
    pub ty: Type,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modifier {
    None,
    Real,
}

#[derive(Debug)]
pub struct Block(pub Vec<Statement>);

#[derive(Debug)]
pub struct Statement {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug)]
pub enum StmtKind {
    Assign(Assignment),
    Assert(Expression),
    Return(Expression),
    Unsafe(Block),
}

#[derive(Debug)]
pub struct Assignment {
    pub modifier: Modifier,
    pub lhs: Id,
    pub rhs: Expression,
}

#[derive(Debug)]
pub struct Expression {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug)]
pub enum ExprKind {
    Id(Id),
    Lit(Literal),
    Field(Box<Expression>, Id),
    Unary(UnaryOp, Box<Expression>),
    Binary(BinaryOp, Box<Expression>, Box<Expression>),
    Call(Box<Call>),
    Record(Box<Constructor>),
}

#[derive(Clone, Copy, Debug)]
pub struct Id {
    pub symbol: Symbol,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Literal {
    pub value: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct UnaryOp {
    pub kind: UnaryKind,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryKind {
    Neg,
    Not,
}

impl UnaryKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Neg => "-",
            Self::Not => "!",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BinaryOp {
    pub kind: BinaryKind,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryKind {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Shl,
    Shr,
    And,
    Or,
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
}

impl BinaryKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
            Self::Pow => "^",
            Self::Shl => "<<",
            Self::Shr => ">>",
            Self::And => "and",
            Self::Or => "or",
            Self::Eq => "=",
            Self::Ne => "!=",
            Self::Gt => ">",
            Self::Ge => ">=",
            Self::Lt => "<",
            Self::Le => "<=",
        }
    }
}

#[derive(Debug)]
pub struct Call {
    pub name: Id,
    pub generics: Vec<Expression>,
    pub args: Vec<Expression>,
}

#[derive(Debug)]
pub struct Constructor {
    pub name: Id,
    pub generics: Vec<Expression>,
    pub fields: Vec<(Id, Expression)>,
}

#[derive(Clone, Copy, Debug)]
pub struct Span(usize, usize);

impl Span {
    #[inline]
    pub fn new(start: usize, end: usize) -> Span {
        Span(start, end)
    }
}

impl From<Range<usize>> for Span {
    #[inline]
    fn from(value: Range<usize>) -> Self {
        Span(value.start, value.end)
    }
}

impl From<Span> for Range<usize> {
    #[inline]
    fn from(value: Span) -> Self {
        value.0..value.1
    }
}
