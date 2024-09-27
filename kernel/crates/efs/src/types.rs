//! Definitions of needed types.
//!
//! See [the POSIX `<sys/types.h>` header](https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/sys_types.h.html) for more information.

use derive_more::{Deref, DerefMut};

/// Used for device IDs.
///
/// It contains a [`u32`], following [the POSIX specification](https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/sys_types.h.html) and [the Linux implementation](https://git.kernel.org/pub/scm/linux/kernel/git/stable/linux.git/tree/include/linux/types.h?h=linux-6.9.y#n21).
#[derive(Debug, Clone, Copy, Deref, DerefMut, Default)]
pub struct Dev(pub u32);

/// Used for file serial numbers.
///
/// It contains a [`usize`], following [the POSIX specification](https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/sys_types.h.html) and [the Linux implementation](https://git.kernel.org/pub/scm/linux/kernel/git/stable/linux.git/tree/include/linux/types.h?h=linux-6.9.y#n22).
#[derive(Debug, Clone, Copy, Deref, DerefMut)]
pub struct Ino(pub usize);

/// Used for some file attributes.
///
/// It contains a [`u16`], following [the POSIX specification](https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/sys_types.h.html) and [the Linux implementation](https://git.kernel.org/pub/scm/linux/kernel/git/stable/linux.git/tree/include/linux/types.h?h=linux-6.9.y#n23) ([`u16`] instead of `short` that doesn't exist in Rust to be compatible with 32-bits systems).
#[derive(Debug, Clone, Copy, Deref, DerefMut)]
pub struct Mode(pub u16);

/// Used for link counts.
///
/// It contains a [`u32`], following [the POSIX specification](https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/sys_types.h.html) and [the Linux implementation](https://git.kernel.org/pub/scm/linux/kernel/git/stable/linux.git/tree/include/linux/types.h?h=linux-6.9.y#n25).
#[derive(Debug, Clone, Copy, Deref, DerefMut)]
pub struct Nlink(pub u32);

/// Used for user IDs.
///
/// It contains a [`u32`], following [the POSIX specification](https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/sys_types.h.html) and [the Linux implementation](https://git.kernel.org/pub/scm/linux/kernel/git/stable/linux.git/tree/include/linux/types.h?h=linux-6.9.y#n37).
#[derive(Debug, Clone, Copy, Deref, DerefMut)]
pub struct Uid(pub u32);

/// Used for group IDs.
///
/// It contains a [`u32`], following [the POSIX specification](https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/sys_types.h.html) and [the Linux implementation](https://git.kernel.org/pub/scm/linux/kernel/git/stable/linux.git/tree/include/linux/types.h?h=linux-6.9.y#n38).
#[derive(Debug, Clone, Copy, Deref, DerefMut)]
pub struct Gid(pub u32);

/// Used for file sizes.
///
/// It contains a [`isize`], following [the POSIX specification](https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/sys_types.h.html) and [the Linux implementation](https://git.kernel.org/pub/scm/linux/kernel/git/stable/linux.git/tree/include/linux/types.h?h=linux-6.9.y#n26).
#[derive(Debug, Clone, Copy, Deref, DerefMut)]
pub struct Off(pub isize);

/// Used for block sizes.
///
/// It contains a [`isize`], following [the POSIX specification](https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/sys_types.h.html) and [the Linux implementation](https://git.kernel.org/pub/scm/linux/kernel/git/stable/linux.git/tree/tools/include/nolibc/std.h?h=linux-6.9.y#n32).
#[derive(Debug, Clone, Copy, Deref, DerefMut)]
pub struct Blksize(pub isize);

/// Used for file block counts.
///
/// It contains a [`i64`], following [the POSIX specification](https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/sys_types.h.html) and [the Linux implementation](https://git.kernel.org/pub/scm/linux/kernel/git/stable/linux.git/tree/include/linux/types.h?h=linux-6.9.y#n26).
#[derive(Debug, Clone, Copy, Deref, DerefMut)]
pub struct Blkcnt(pub i64);

/// Used for time in seconds.
#[derive(Debug, Clone, Copy, Deref, DerefMut)]
pub struct Time(pub i64);

/// Used for precise instants.
///
/// Times shall be given in seconds since the Epoch. If possible, it can be completed with nanoseconds.
#[derive(Debug, Clone, Copy)]
pub struct Timespec {
    /// Whole seconds.
    pub tv_sec: Time,

    /// Nanoseconds [0, 999999999].
    pub tv_nsec: u32,
}
