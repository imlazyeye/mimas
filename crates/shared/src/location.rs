use std::ops::Range;

pub type FileId = usize;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const SYNTHETIC: Self = Self {
        start: u32::MAX as usize,
        end: u32::MAX as usize,
    };

    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn start(&self) -> usize {
        self.start
    }

    pub fn end(&self) -> usize {
        self.end
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    pub fn is_synthetic(&self) -> bool {
        *self == Self::SYNTHETIC
    }

    pub fn until(&self, other: Self) -> Self {
        Self::new(self.start, other.end)
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Location {
    pub file_id: FileId,
    pub span: Span,
}

impl Location {
    pub const SYNTHETIC: Self = Self {
        file_id: u32::MAX as usize,
        span: Span::SYNTHETIC,
    };

    pub fn new(file_id: FileId, span: impl Into<Span>) -> Self {
        Self {
            file_id,
            span: span.into(),
        }
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn file_id(&self) -> FileId {
        self.file_id
    }

    pub fn is_synthetic(&self) -> bool {
        *self == Self::SYNTHETIC
    }
}

/// Trait for types that carry a `Location`. Mirrors chompy's `Located` for the methods mimas
/// actually uses (no diagnostic-label helpers; those belong on the kinds themselves now).
pub trait Located {
    fn location(&self) -> Location;

    fn span(&self) -> Span {
        self.location().span
    }

    fn file_id(&self) -> FileId {
        self.location().file_id
    }
}

impl Located for Location {
    fn location(&self) -> Location {
        *self
    }
}

impl From<Span> for Range<usize> {
    fn from(s: Span) -> Self {
        s.start..s.end
    }
}

impl From<Range<usize>> for Span {
    fn from(r: Range<usize>) -> Self {
        Self::new(r.start, r.end)
    }
}

impl From<chompy::utils::Span> for Span {
    fn from(s: chompy::utils::Span) -> Self {
        Self::new(s.start(), s.end())
    }
}

impl From<chompy::utils::Location> for Location {
    fn from(l: chompy::utils::Location) -> Self {
        Self {
            file_id: l.file_id(),
            span: l.span().into(),
        }
    }
}

impl From<Span> for chompy::utils::Span {
    fn from(s: Span) -> Self {
        chompy::utils::Span::new(s.start, s.end)
    }
}

impl From<Location> for chompy::utils::Location {
    fn from(l: Location) -> Self {
        chompy::utils::Location::new(l.file_id, l.span)
    }
}

impl From<Span> for miette::SourceSpan {
    fn from(s: Span) -> Self {
        miette::SourceSpan::new(s.start.into(), s.end - s.start)
    }
}

impl From<Location> for miette::SourceSpan {
    fn from(l: Location) -> Self {
        l.span.into()
    }
}
