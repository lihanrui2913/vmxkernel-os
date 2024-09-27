//! General description of sectors.

use core::fmt::Debug;
use core::iter::Step;
use core::ops::Mul;

use derive_more::{Add, Deref, DerefMut, LowerHex, Sub};

#[cfg(target_pointer_width = "32")]
use crate::arch::usize_to_u32;
#[cfg(target_pointer_width = "64")]
use crate::arch::usize_to_u64;
use crate::arch::{u32_to_usize, u64_to_usize};

/// Address of a physical sector
#[derive(Debug, Clone, Copy, PartialEq, Eq, LowerHex, PartialOrd, Ord, Deref, DerefMut, Add, Sub)]
pub struct Address(usize);

impl Address {
    /// Returns a new [`Address`] from its index.
    ///
    /// This function is equivalent to the [`From<usize>`](struct.Address.html#impl-From<usize>-for-Address) implementation but
    /// with a `const fn`.
    #[must_use]
    pub const fn new(index: usize) -> Self {
        Self(index)
    }

    /// Returns the index of this address, which corresponds to its offset from the start of the device.
    #[must_use]
    pub const fn index(&self) -> usize {
        self.0
    }
}

impl From<usize> for Address {
    fn from(index: usize) -> Self {
        Self(index)
    }
}

impl From<Address> for usize {
    fn from(value: Address) -> Self {
        value.0
    }
}

#[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
impl From<u32> for Address {
    fn from(value: u32) -> Self {
        Self(u32_to_usize(value))
    }
}

#[cfg(target_pointer_width = "32")]
impl From<Address> for u32 {
    fn from(value: Address) -> Self {
        usize_to_u32(value.0)
    }
}

#[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
impl From<u64> for Address {
    fn from(value: u64) -> Self {
        Self(u64_to_usize(value))
    }
}

#[cfg(target_pointer_width = "64")]
impl From<Address> for u64 {
    fn from(value: Address) -> Self {
        usize_to_u64(value.0)
    }
}

impl Add<usize> for Address {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self(*self + rhs)
    }
}

impl Sub<usize> for Address {
    type Output = Self;

    fn sub(self, rhs: usize) -> Self::Output {
        Self(*self - rhs)
    }
}

impl Mul<usize> for Address {
    type Output = Self;

    fn mul(self, rhs: usize) -> Self::Output {
        Self(*self * rhs)
    }
}

impl Step for Address {
    fn steps_between(start: &Self, end: &Self) -> Option<usize> {
        usize::steps_between(start, end)
    }

    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        usize::forward_checked(*start, count).map(Into::into)
    }

    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        usize::backward_checked(*start, count).map(Into::into)
    }
}
