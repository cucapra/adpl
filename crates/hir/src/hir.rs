pub use adpl_arena::{Index, IndexRange, List, NonMaxIndex};
pub use adpl_ast::{
    BinaryKind, BinaryOp, Id, Literal, Safety, Span, Symbol, UnaryKind, UnaryOp,
};

#[derive(Debug)]
pub struct Record {
    pub name: Id,
    pub params: IndexRange<Local>,
    pub fields: IndexRange<Field>,
}

#[derive(Debug)]
pub struct Field {
    pub name: Id,
    pub ty: Index<Type>,
    pub span: Span,
}

#[derive(Debug)]
pub struct Type {
    pub name: Id,
    pub args: List<Expression>,
    pub span: Span,
}

#[derive(Debug)]
pub struct Definition {
    pub safety: Safety,
    pub name: Id,
    pub generics: IndexRange<Local>,
    pub requires: Option<NonMaxIndex<Expression>>,
    pub implements: Option<NonMaxIndex<Expression>>,
    pub inputs: IndexRange<Parameter>,
    pub output: Index<Type>,
    pub body: Option<List<Statement>>,
}

#[derive(Debug)]
pub struct Parameter {
    pub local: Index<Local>,
    pub ty: Index<Type>,
    pub span: Span,
}

#[derive(Debug)]
pub struct Statement {
    pub kind: StmtKind,
}

#[derive(Debug)]
pub enum StmtKind {
    Assign(Index<Local>, Index<Expression>),
    Return(Index<Expression>),
    Unsafe(List<Statement>),
}

#[derive(Debug)]
pub struct Expression {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug)]
pub enum ExprKind {
    Id(Index<Local>),
    Lit(Literal),
    Field(Index<Expression>, Id),
    Unary(UnaryOp, Index<Expression>),
    Binary(BinaryOp, Index<Expression>, Index<Expression>),
    Call(Call),
    Record(Constructor),
}

#[derive(Debug)]
pub struct Local {
    pub kind: LocalKind,
    pub name: Id,
}

#[derive(Debug)]
pub enum LocalKind {
    Let(Index<Expression>),
    Param(u16),
    GenericParam(u16),
}

#[derive(Debug)]
pub struct Call {
    pub name: Id,
    pub callee: Index<Definition>,
    pub generics: List<Expression>,
    pub args: List<Expression>,
}

#[derive(Debug)]
pub struct Constructor {
    pub name: Id,
    pub record: Index<Record>,
    pub generics: List<Expression>,
    pub inits: List<Expression>,
}
