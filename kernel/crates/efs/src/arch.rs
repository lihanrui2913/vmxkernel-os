//! Architecture related functions, such as integer conversions.

/// Converts a [`usize`] into a [`u32`].
///
/// # Panics
///
/// Panics if the target pointer width is strictly greater than 32-bits.
#[inline]
#[must_use]
pub fn usize_to_u32(#[allow(unused)] n: usize) -> u32 {
    cfg_if::cfg_if! {
        if #[cfg(any(target_pointer_width = "16", target_pointer_width = "32"))] {
            // SAFETY: n is a u32 because usize <= u32
            unsafe {
                u32::try_from(n).unwrap_unchecked()
            }
        } else {
            panic!("Cannot convert an integer encoded on more than 32 bits into a u32");
        }
    }
}

/// Converts a [`usize`] into a [`u64`].
///
/// # Panics
///
/// Panics if the target pointer width is strictly greater than 64-bits.
#[inline]
#[must_use]
pub fn usize_to_u64(#[allow(unused)] n: usize) -> u64 {
    cfg_if::cfg_if! {
        if #[cfg(any(target_pointer_width = "16", target_pointer_width = "32", target_pointer_width = "64"))] {
            // SAFETY: n is a u64 because usize <= u64
            unsafe {
                u64::try_from(n).unwrap_unchecked()
            }
        } else {
            panic!("Cannot convert an integer encoded on more than 64 bits into a u64");
        }
    }
}

/// Converts a [`u32`] into a [`usize`].
///
/// # Panics
///
/// Panics if the target pointer width is strictly smaller than 32-bits.
#[inline]
#[must_use]
pub fn u32_to_usize(#[allow(unused)] n: u32) -> usize {
    cfg_if::cfg_if! {
        if #[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))] {
            // SAFETY: n is a usize because u32 <= usize
            unsafe {
                usize::try_from(n).unwrap_unchecked()
            }
        } else {
            panic!("Cannot convert an integer encoded on 32 bits into a usize");
        }
    }
}

/// Converts a [`u64`] into a [`usize`].
///
/// # Panics
///
/// Panics if the target pointer width is strictly smaller than 64-bits.
#[inline]
#[must_use]
pub fn u64_to_usize(#[allow(unused)] n: u64) -> usize {
    cfg_if::cfg_if! {
        if #[cfg(any(target_pointer_width = "64"))] {
            // SAFETY: n is a usize because u64 == usize
            unsafe {
                usize::try_from(n).unwrap_unchecked()
            }
        } else {
            panic!("Cannot convert an integer encoded on 64 bits into a usize");
        }
    }
}
