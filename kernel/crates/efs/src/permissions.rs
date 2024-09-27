//! Interface for UNIX permissions.
//!
//! See [this Wikipedia page](https://en.wikipedia.org/wiki/File-system_permissions#Notation_of_traditional_Unix_permissions) and [this POSIX definition](https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/V1_chap04.html#tag_04_07).

use core::fmt::Display;

use bitflags::bitflags;

use crate::types::Mode;

/// Represents the three permission classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// Owner of the file.
    Owner,

    /// Group members of the file.
    Group,

    /// Other users.
    Other,
}

/// Represents a triad of permissions.
#[derive(Debug, Clone, Copy)]
pub struct Triad {
    /// Read permission.
    pub read: bool,

    /// Write permission.
    pub write: bool,

    /// Execution permission.
    pub execution: bool,
}

bitflags! {
    /// Permissions bits.
    ///
    /// The permission indicator occupies the bottom 12 bits.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Permissions: u16 {
        /// Other - execute permission
        const OTHER_EXECUTION   =   0o000_001;

        /// Other - write permission
        const OTHER_WRITE       =   0o000_002;

        /// Other - read permission
        const OTHER_READ        =   0o000_004;

        /// Group - execute permission
        const GROUP_EXECUTION   =   0o000_010;

        /// Group - write permission
        const GROUP_WRITE       =   0o000_020;

        /// Group - read permission
        const GROUP_READ        =   0o000_040;

        /// User - execute permission
        const USER_EXECUTION    =   0o000_100;

        /// User - write permission
        const USER_WRITE        =   0o000_200;

        /// User - read permission
        const USER_READ         =   0o000_400;

        /// Sticky bit
        const STICKY            =   0o001_000;

        /// Set group ID
        const SET_GROUP_ID      =   0o002_000;

        /// Set user ID
        const SET_USER_ID       =   0o004_000;
    }
}

impl Display for Permissions {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            fmt,
            "{}{}{}{}{}{}{}{}{}",
            if self.contains(Self::USER_READ) { "r" } else { "-" },
            if self.contains(Self::USER_WRITE) { "w" } else { "-" },
            if self.contains(Self::USER_WRITE) && self.contains(Self::SET_USER_ID) {
                "s"
            } else if self.contains(Self::SET_USER_ID) {
                "S"
            } else if self.contains(Self::USER_EXECUTION) {
                "x"
            } else {
                "-"
            },
            if self.contains(Self::GROUP_READ) { "r" } else { "-" },
            if self.contains(Self::GROUP_WRITE) { "w" } else { "-" },
            if self.contains(Self::GROUP_WRITE) && self.contains(Self::SET_GROUP_ID) {
                "s"
            } else if self.contains(Self::SET_GROUP_ID) {
                "S"
            } else if self.contains(Self::GROUP_EXECUTION) {
                "x"
            } else {
                "-"
            },
            if self.contains(Self::OTHER_READ) { "r" } else { "-" },
            if self.contains(Self::OTHER_WRITE) { "w" } else { "-" },
            if self.contains(Self::OTHER_WRITE) { "x" } else { "-" }
        )
    }
}

impl From<Mode> for Permissions {
    fn from(value: Mode) -> Self {
        Self::from_bits_truncate(value.0)
    }
}

impl From<Permissions> for Mode {
    fn from(value: Permissions) -> Self {
        Self(value.bits())
    }
}

impl Permissions {
    /// Returns the permission [`Triad`] for the given [`Class`].
    #[must_use]
    pub const fn triad_for(&self, class: Class) -> Triad {
        let (read_class, write_class, execution_class) = match class {
            Class::Owner => (Self::USER_READ, Self::USER_WRITE, Self::USER_EXECUTION),
            Class::Group => (Self::GROUP_READ, Self::GROUP_WRITE, Self::GROUP_EXECUTION),
            Class::Other => (Self::OTHER_READ, Self::OTHER_WRITE, Self::OTHER_EXECUTION),
        };

        Triad {
            read: self.contains(read_class),
            write: self.contains(write_class),
            execution: self.contains(execution_class),
        }
    }
}
