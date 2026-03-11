use adpl_hir as hir;

#[derive(Clone, Copy)]
pub enum Primitive {
    Bool,
    Int,
    UInt,
    Ieee,
}

impl Primitive {
    pub const VARIANTS: [Self; 4] = [
        Primitive::Bool,
        Primitive::Int,
        Primitive::UInt,
        Primitive::Ieee,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Primitive::Bool => "bool",
            Primitive::Int => "int",
            Primitive::UInt => "uint",
            Primitive::Ieee => "ieee",
        }
    }

    pub fn params(self) -> usize {
        match self {
            Primitive::Bool => 0,
            Primitive::Int | Primitive::UInt => 1,
            Primitive::Ieee => 2,
        }
    }
}

impl From<Primitive> for hir::TypeKind {
    fn from(value: Primitive) -> Self {
        match value {
            Primitive::Bool => hir::TypeKind::Bool,
            Primitive::Int => hir::TypeKind::Int,
            Primitive::UInt => hir::TypeKind::UInt,
            Primitive::Ieee => hir::TypeKind::Ieee,
        }
    }
}
