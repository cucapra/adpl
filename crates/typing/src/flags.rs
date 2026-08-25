use std::ops;

use crate::types::TypeArenas;
use crate::visit::Visitor;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ParamFlags(u8);

impl ParamFlags {
    pub const IN_IO_TYPE: Self = ParamFlags(1 << 0);
    pub const IN_PRECONDITION: Self = ParamFlags(1 << 1);
    pub const IN_SPECIFICATION: Self = ParamFlags(1 << 2);

    #[inline]
    pub const fn empty() -> Self {
        ParamFlags(0)
    }

    #[inline]
    pub const fn contains(self, flags: Self) -> bool {
        self.0 & flags.0 == flags.0
    }
}

impl ops::BitOr for ParamFlags {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self {
        ParamFlags(self.0 | rhs.0)
    }
}

impl ops::BitOrAssign for ParamFlags {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

pub struct SetParamFlags<'a> {
    pub ctx: &'a TypeArenas,
    pub out: &'a mut [ParamFlags],
    pub flags: ParamFlags,
}

impl<'a> Visitor<'a, TypeArenas> for SetParamFlags<'a> {
    type Result = ();

    fn ctx(&self) -> &'a TypeArenas {
        self.ctx
    }

    fn visit_param(&mut self, index: usize) {
        self.out[index] |= self.flags;
    }
}
