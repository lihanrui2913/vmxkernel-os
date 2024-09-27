//! Interface for `efs` possible errors

use alloc::string::String;
#[cfg(feature = "std")]
use alloc::string::ToString;

use derive_more::derive::{Display, From};

use crate::dev::error::DevError;
use crate::fs::error::FsError;
use crate::path::PathError;

/// Enumeration of possible sources of error
#[allow(clippy::error_impl_error)]
#[derive(Debug, Display, From)]
#[display("Error: {_variant}")]
pub enum Error<FSE: core::error::Error> {
    /// Device error
    Device(DevError),

    /// Filesystem error
    Fs(FsError<FSE>),

    /// Path error
    Path(PathError),

    /// I/O error
    IO(String),
}

impl<FSE: core::error::Error> core::error::Error for Error<FSE> {}

#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
impl<FSE: core::error::Error> From<std::io::Error> for Error<FSE> {
    fn from(value: std::io::Error) -> Self {
        match value.kind() {
            std::io::ErrorKind::UnexpectedEof => Self::Device(DevError::UnexpectedEof),
            std::io::ErrorKind::WriteZero => Self::Device(DevError::WriteZero),
            _ => Self::IO(value.to_string()),
        }
    }
}
