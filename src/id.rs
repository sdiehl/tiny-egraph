//! E-class identifier.

use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(u32);

impl Id {
    #[inline]
    #[must_use]
    pub const fn from_u32(n: u32) -> Self {
        Self(n)
    }

    #[inline]
    #[must_use]
    pub const fn index(self) -> usize {
        self.0 as usize
    }

    #[inline]
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

impl fmt::Debug for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Id({})", self.0)
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<usize> for Id {
    fn from(n: usize) -> Self {
        Self(u32::try_from(n).expect("Id index overflows u32"))
    }
}

impl From<u32> for Id {
    fn from(n: u32) -> Self {
        Self(n)
    }
}

impl From<Id> for usize {
    fn from(id: Id) -> Self {
        id.0 as Self
    }
}

impl From<Id> for u32 {
    fn from(id: Id) -> Self {
        id.0
    }
}
