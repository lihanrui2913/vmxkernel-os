//! General traits for I/O interfaces.

#[cfg(feature = "std")]
use core::marker::PhantomData;

use crate::dev::error::DevError;
use crate::error::Error;

/// Base I/O trait that must be implemented for all types implementing [`Read`], [`Write`] or [`Seek`].
pub trait Base {
    /// Error type corresponding to the [`FileSystem`](crate::fs::FileSystem) implemented.
    type FsError: core::error::Error;
}

/// Allows for reading bytes from a source.
///
/// See [`std::io::Read`] for more information: this trait is a `no_std` based variant.
pub trait Read: Base {
    /// Pull some bytes from this source into the specified buffer, returning how many bytes were read.
    ///
    /// If the returned number is 0, the reader is considered as ended.
    ///
    /// On a [`Seek`]able reader, a call to this function should increase the offset by the amount of bytes read.
    ///
    /// See [`read`](https://docs.rs/no_std_io/latest/no_std_io/io/trait.Read.html#tymethod.read) for more information.
    ///
    /// # Errors
    ///
    /// Returns an [`DevError`] if the device on which the directory is located could not be read.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error<Self::FsError>>;

    /// Read the exact number of bytes required to fill buf.
    ///
    /// See [`read_exact`](https://docs.rs/no_std_io/latest/no_std_io/io/trait.Read.html#method.read_exact) for more information.
    ///
    /// # Errors
    ///
    /// Returns an [`UnexpectedEof`](DevError::UnexpectedEof) if the buffer could not be entirely filled.
    ///
    /// Otherwise, returns the same errors as [`read`](Read::read).
    fn read_exact(&mut self, mut buf: &mut [u8]) -> Result<(), Error<Self::FsError>> {
        while !buf.is_empty() {
            match self.read(buf) {
                Ok(0) => break,
                Ok(n) => {
                    let tmp = buf;
                    buf = &mut tmp[n..];
                },
                Err(err) => return Err(err),
            }
        }
        if buf.is_empty() { Ok(()) } else { Err(Error::Device(DevError::UnexpectedEof)) }
    }
}

/// Allows for writing bytes to a destination.
///
/// See [`std::io::Write`] for more information: this trait is a `no_std` based variant.
pub trait Write: Base {
    /// Write a buffer into this writer, returning how many bytes were written.
    ///
    /// If the returned number is 0, either the writer is ended or cannot add any more bytes at its end.
    ///
    /// On a [`Seek`]able writer, a call to this function should increase the offset by the amount of bytes read.
    ///
    /// See [`write`](https://docs.rs/no_std_io/latest/no_std_io/io/trait.Write.html#tymethod.write) for more information.
    ///
    /// # Errors
    ///
    /// Returns an [`DevError`] if the device on which the directory is located could not be written.
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error<Self::FsError>>;

    /// Flush this output stream, ensuring that all intermediately buffered contents reach their destination.
    ///
    /// See [`flush`](https://docs.rs/no_std_io/latest/no_std_io/io/trait.Write.html#tymethod.flush) for more information.
    ///
    /// # Errors
    ///
    /// Returns an [`DevError`] if the device on which the directory is located could not be read.
    fn flush(&mut self) -> Result<(), Error<Self::FsError>>;

    /// Attempts to write an entire buffer into this writer.
    ///
    /// See [`write_all`](https://docs.rs/no_std_io/latest/no_std_io/io/trait.Write.html#method.write_all) for more information.
    ///
    /// # Errors
    ///
    /// Returns a [`WriteZero`](DevError::WriteZero) error if the buffer could not be written entirely.
    ///
    /// Otherwise, returns the same errors as [`write`](Write::write).
    #[allow(clippy::indexing_slicing)]

    fn write_all(&mut self, mut buf: &[u8]) -> Result<(), Error<Self::FsError>> {
        while !buf.is_empty() {
            match self.write(buf) {
                Ok(0) => {
                    return Err(Error::Device(DevError::WriteZero));
                },
                Ok(n) => buf = &buf[n..],
                Err(err) => return Err(err),
            }
        }
        Ok(())
    }
}

/// Enumeration of possible methods to seek within an I/O object.
///
/// See [`std::io::SeekFrom`] for more information.
#[derive(Debug, Clone, Copy)]
pub enum SeekFrom {
    /// Sets the offset to the provided number of bytes.
    Start(u64),

    /// Sets the offset to the size of this object plus the specified number of bytes.
    ///
    /// It is possible to seek beyond the end of an object, but it’s an error to seek before byte 0.
    End(i64),

    /// Sets the offset to the current position plus the specified number of bytes.
    ///
    /// It is possible to seek beyond the end of an object, but it’s an error to seek before byte 0.
    Current(i64),
}

#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
impl From<std::io::SeekFrom> for SeekFrom {
    fn from(value: std::io::SeekFrom) -> Self {
        match value {
            std::io::SeekFrom::Start(value) => Self::Start(value),
            std::io::SeekFrom::End(value) => Self::End(value),
            std::io::SeekFrom::Current(value) => Self::Current(value),
        }
    }
}

#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
impl From<SeekFrom> for std::io::SeekFrom {
    fn from(value: SeekFrom) -> Self {
        match value {
            SeekFrom::Start(value) => Self::Start(value),
            SeekFrom::End(value) => Self::End(value),
            SeekFrom::Current(value) => Self::Current(value),
        }
    }
}

/// Provides a cursor which can be moved within a stream of bytes.
///
/// See [`std::io::Seek`] for more information: this trait is a `no_std` based variant.
pub trait Seek: Base {
    /// Seek to an offset, in bytes, in a stream.
    ///
    /// See [`seek`](https://docs.rs/no_std_io/latest/no_std_io/io/trait.Seek.html#tymethod.seek) for more information.
    ///
    /// # Errors
    ///
    /// Returns an [`DevError`] if the device on which the directory is located could not be read.
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Error<Self::FsError>>;
}

/// A wrapper struct for types that have implementations for [`std::io`] traits.
///
/// [`Read`], [`Write`] and [`Seek`] are implemented for this type if the corresponding [`std::io`] trait is implemented for `S`.
///
/// Moreover, `FSE` correspond to the filesystem's error type.
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub struct StdIOWrapper<S, FSE: core::error::Error> {
    /// Inner object, supposedly implementing at least one [`std::io`] trait.
    inner: S,

    /// Phantom data for the file system error.
    phantom: PhantomData<FSE>,
}

#[cfg(feature = "std")]
impl<S, FSE: core::error::Error> StdIOWrapper<S, FSE> {
    /// Creates an [`StdIOWrapper`] from the object it wraps.
    #[must_use]
    pub const fn new(inner: S) -> Self {
        Self {
            inner,
            phantom: PhantomData,
        }
    }
}

#[cfg(feature = "std")]
impl<S, FSE: core::error::Error> Base for StdIOWrapper<S, FSE> {
    type FsError = FSE;
}

#[cfg(feature = "std")]
impl<S: std::io::Read, FSE: core::error::Error> Read for StdIOWrapper<S, FSE> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error<Self::FsError>> {
        let res = self.inner.read(buf)?;
        Ok(res)
    }
}

#[cfg(feature = "std")]
impl<S: std::io::Write, FSE: core::error::Error> Write for StdIOWrapper<S, FSE> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error<Self::FsError>> {
        let res = self.inner.write(buf)?;
        Ok(res)
    }

    fn flush(&mut self) -> Result<(), Error<Self::FsError>> {
        self.inner.flush()?;
        Ok(())
    }
}

#[cfg(feature = "std")]
impl<S: std::io::Seek, FSE: core::error::Error> Seek for StdIOWrapper<S, FSE> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Error<Self::FsError>> {
        let res = self.inner.seek(pos.into())?;
        Ok(res)
    }
}

#[cfg(feature = "std")]
impl<S, FSE: core::error::Error> From<S> for StdIOWrapper<S, FSE> {
    fn from(value: S) -> Self {
        Self::new(value)
    }
}
