use std::ops::Range;

#[derive(Clone, Copy, Debug)]
pub struct Span {
    start: u32,
    len: u16,
    file: u16,
}

impl Span {
    pub fn new(file: u16, range: Range<usize>) -> Span {
        let len = range.end - range.start;

        Span {
            start: range.start.try_into().unwrap(),
            len: len.try_into().unwrap_or(u16::MAX),
            file,
        }
    }

    #[inline]
    pub fn from_pair(pair: (u16, Range<usize>)) -> Span {
        Span::new(pair.0, pair.1)
    }

    #[inline]
    pub fn file(self) -> usize {
        self.file.into()
    }

    #[inline]
    pub fn range(self) -> Range<usize> {
        let start = self.start as usize;
        let end = start + self.len as usize;

        start..end
    }
}

impl From<Span> for (usize, Range<usize>) {
    #[inline]
    fn from(value: Span) -> Self {
        (value.file(), value.range())
    }
}
