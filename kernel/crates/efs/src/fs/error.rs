//! Errors related to filesystems manipulation.

use alloc::string::String;

use derive_more::derive::Display;

use crate::file::Type;

/// Enumeration of possible errors encountered with [`FileSystem`](super::FileSystem)s' manipulation.
#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Display)]
#[display("FileSystem Error: {_variant}")]
pub enum FsError<E: core::error::Error> {
    /// Indicates that the given [`File`](crate::file::File) already exist in the given directory.
    #[display("Entry Already Exist: \"{_0}\" already exist in given directory")]
    EntryAlreadyExist(String),

    /// Indicates that this error is coming from the filesystem's implementation.
    #[display("Implementation: {_0}")]
    Implementation(E),

    /// Indicates that a loop has been encountered during the given path resolution.
    #[display("Loop: a loop has been encountered during the resolution of \"{_0}\"")]
    Loop(String),

    /// Indicates that the given [`Path`](crate::path::Path) is too long to be resolved.
    #[display("Name too long: \"{_0}\" is too long to be resolved")]
    NameTooLong(String),

    /// Indicates that the given filename is not a [`Directory`](crate::file::Directory).
    #[display("Not a Directory: \"{_0}\" is not a directory")]
    NotDir(String),

    /// Indicates that the given filename is an symbolic link pointing at an empty string.
    #[display("No Entry: \"{_0}\" is an symbolic link pointing at an empty string")]
    NoEnt(String),

    /// Indicates that the given filename has not been found.
    #[display("Not Found: \"{_0}\" has not been found")]
    NotFound(String),

    /// Tried to remove the current directory or a parent directory, which is not permitted.
    #[display("Remove Refused: Tried to remove the current directory or a parent directory, which is not permitted")]
    RemoveRefused,

    /// Tried to assign a wrong type to a file.
    #[display("Wrong File Type: {expected:?} file type expected, {given:?} given")]
    WrongFileType {
        /// Expected file type.
        expected: Type,

        /// Given file type.
        given: Type,
    },
}

impl<FSE: core::error::Error> core::error::Error for FsError<FSE> {}
