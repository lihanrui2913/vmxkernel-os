//! General interface for filesystems.
//!
//! # Filesystems
//!
//! In this crate, a [`FileSystem`] is a structure capable of managing a complete filesystem on a given
//! [`Device`](crate::dev::Device). A [`FileSystem`] is the entry point to the [`File`](crate::file::File)s ([regular
//! files](crate::file::Regular), [directories](crate::file::Directory), ...) of your filesystem through the method
//! [`root`](FileSystem::root).
//!
//! The other provided methods, such as [`get_file`](FileSystem::get_file), are here to help you read and manipulate easily the
//! content of your filesystem.
//!
//! All the needed methods for [`File`](crate::file::File) manipulation are implemented by other structures linked to a
//! [`FileSystem`]. You can read the module [`file`](crate::file)'s documentation for more information.
//!
//! ## How to implement a filesystem?
//!
//! To implement a filesystem, you will need a lot of structures and methods. You can read the implementation of the `ext2`
//! filesystem as an example, but here is a general layout of what you need to do:
//!
//! * create a structure which will implement [`FileSystem`]: it will be the core structure of your filesystem
//!
//! * create an error structure, which implements [`core::error::Error`]. This will contain **every** error that your filesystem
//!   will be able to return.
//!
//! * create objects for every structure in your filesystem
//!
//! * create structures for [`File`](crate::file::File), [`Regular`](crate::file::Regular), [`Directory`] and [`SymbolicLink`]. For
//!   each of this structure, create functions allowing to be parsed easily. For [`Fifo`](crate::file::Fifo),
//!   [`CharacterDevice`](crate::file::CharacterDevice), [`BlockDevice`](crate::file::BlockDevice) and
//!   [`Socket`](crate::file::Socket), you can use a simple struct like `struct Socket(File)` as you will likely never use them
//!   directly with this crate
//!
//! * implement the functions allowing to retrieve the [`Regular`](crate::file::Regular), [`Directory`] and [`SymbolicLink`], and
//!   the [`root`](FileSystem::root) particularily. For the [`double_slash_root`](FileSystem::double_slash_root), if you don't know
//!   what it means, you can just implement it as `self.root()` (and it will very probably be the right thing to do)
//!
//! * implements all the other functions for the [`Regular`](crate::file::Regular), [`Directory`] and [`SymbolicLink`] structures.
//!
//! Advice: start with the read-only functions and methods. It will be **MUCH** easier that the write methods.

use alloc::borrow::ToOwned;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::str::FromStr;

use itertools::{Itertools, Position};

use crate::error::Error;
use crate::file::{Directory, ReadOnlyDirectory, ReadOnlySymbolicLink, ReadOnlyTypeWithFile, SymbolicLink, Type, TypeWithFile};
use crate::fs::error::FsError;
use crate::path::{Component, Path, PathError};
use crate::permissions::Permissions;
use crate::types::{Gid, Uid};

pub mod error;
pub mod structures;

#[cfg(feature = "ext2")]
#[cfg_attr(docsrs, doc(cfg(feature = "ext2")))]
pub mod ext2;

/// Maximal length for a path.
///
/// This is defined in [this POSIX definition](https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/V1_chap04.html#tag_04_16).
///
/// This value is the same as the one defined in [the linux's `limits.h` header](https://git.kernel.org/pub/scm/linux/kernel/git/stable/linux.git/tree/include/uapi/linux/limits.h?h=linux-6.9.y#n13).
pub const PATH_MAX: usize = 4_096;

/// A filesystem.
pub trait FileSystem<Dir: Directory> {
    /// Returns the root directory of the filesystem.
    ///
    /// # Errors
    ///
    /// Returns a [`Error::Device`] if the device could not be read.
    ///
    /// Otherwise, returns a [`FsError::Implementation`] in any other case.
    fn root(&self) -> Result<Dir, Error<Dir::FsError>>;

    /// Returns the double slash root directory of the filesystem.
    ///
    /// If you do not have any idea of what this is, you are probably looking for [`root`](FileSystem::root).
    ///
    /// See [`DoubleSlashRootDir`](Component::DoubleSlashRootDir) and [`Path`] for more information.
    ///
    /// # Errors
    ///
    /// Returns a [`Error::Device`] if the device could not be read.
    ///
    /// Otherwise, returns a [`FsError::Implementation`] in any other case.
    fn double_slash_root(&self) -> Result<Dir, Error<Dir::FsError>>;

    /// Performs a pathname resolution as described in [this POSIX definition](https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/V1_chap04.html#tag_04_16).
    ///
    /// Returns the file of this filesystem corresponding to the given `path`, starting at the `current_dir`.
    ///
    /// `symlink_resolution` indicates whether the function calling this method is required to act on the symbolic link itself, or
    /// certain arguments direct that the function act on the symbolic link itself.
    ///
    /// # Errors
    ///
    /// Returns a [`Error::Device`] if the device could not be read.
    ///
    /// Returns an [`NotFound`](FsError::NotFound) error if the given path does not leed to an existing path.
    ///
    /// Returns an [`NotDir`](FsError::NotDir) error if one of the components of the file is not a directory.
    ///
    /// Returns an [`Loop`](FsError::Loop) error if a loop is found during the symbolic link resolution.
    ///
    /// Returns an [`NameTooLong`](FsError::NameTooLong) error if the complete path contains more than [`PATH_MAX`] characters.
    ///
    /// Returns an [`NoEnt`](FsError::NoEnt) error if an encountered symlink points to a non-existing file.
    ///
    /// Otherwise, returns a [`FsError::Implementation`] in any other case.
    fn get_file(&self, path: &Path, current_dir: Dir, symlink_resolution: bool) -> Result<TypeWithFile<Dir>, Error<Dir::FsError>>
    where
        Self: Sized,
    {
        /// Auxiliary function used to store the visited symlinks during the pathname resolution to detect loops caused bt symbolic
        /// links.
        fn path_resolution<FSE: core::error::Error, D: Directory<FsError = FSE>>(
            fs: &impl FileSystem<D>,
            path: &Path,
            mut current_dir: D,
            symlink_resolution: bool,
            mut visited_symlinks: Vec<String>,
        ) -> Result<TypeWithFile<D>, Error<FSE>> {
            let canonical_path = path.canonical();

            if canonical_path.len() > PATH_MAX {
                return Err(Error::Fs(FsError::NameTooLong(canonical_path.to_string())));
            }

            let trailing_blackslash = canonical_path.as_unix_str().has_trailing_backslash();
            let mut symlink_encountered = None;

            let mut components = canonical_path.components();

            for (pos, comp) in components.with_position() {
                match comp {
                    Component::RootDir => {
                        if pos == Position::First || pos == Position::Only {
                            current_dir = fs.root()?;
                        } else {
                            unreachable!("The root directory cannot be encountered during the pathname resolution");
                        }
                    },
                    Component::DoubleSlashRootDir => {
                        if pos == Position::First || pos == Position::Only {
                            current_dir = fs.double_slash_root()?;
                        } else {
                            unreachable!("The double slash root directory cannot be encountered during the pathname resolution");
                        }
                    },
                    Component::CurDir => {},
                    Component::ParentDir => {
                        current_dir = current_dir.parent()?;
                    },
                    Component::Normal(filename) => {
                        let children = current_dir.entries()?;
                        let Some(entry) = children.into_iter().find(|entry| entry.filename == filename).map(|entry| entry.file)
                        else {
                            return Err(Error::Fs(FsError::NotFound(filename.to_string())));
                        };

                        #[allow(clippy::wildcard_enum_match_arm)]
                        match entry {
                            TypeWithFile::Directory(dir) => {
                                current_dir = dir;
                            },

                            // This case is the symbolic link resolution, which is the one described as **not** being the one
                            // explained in the following paragraph from the POSIX definition of the pathname resolution:
                            //
                            // If a symbolic link is encountered during pathname resolution, the behavior shall depend on whether
                            // the pathname component is at the end of the pathname and on the function
                            // being performed. If all of the following are true, then pathname
                            // resolution is complete:
                            //   1. This is the last pathname component of the pathname.
                            //   2. The pathname has no trailing <slash>.
                            //   3. The function is required to act on the symbolic link itself, or certain arguments direct that
                            //      the function act on the symbolic link itself.
                            TypeWithFile::SymbolicLink(symlink)
                                if (pos != Position::Last && pos != Position::Only && !trailing_blackslash)
                                    || symlink_resolution =>
                            {
                                let pointed_file = SymbolicLink::get_pointed_file(&symlink)?.to_owned();
                                if pointed_file.is_empty() {
                                    return Err(Error::Fs(FsError::NoEnt(filename.to_string())));
                                };

                                symlink_encountered = Some(pointed_file);
                                break;
                            },
                            _ => {
                                return if (pos == Position::Last || pos == Position::Only) && !trailing_blackslash {
                                    Ok(entry)
                                } else {
                                    Err(Error::Fs(FsError::NotDir(filename.to_string())))
                                };
                            },
                        }
                    },
                }
            }

            match symlink_encountered {
                None => Ok(TypeWithFile::Directory(current_dir)),
                Some(pointed_file) => {
                    if visited_symlinks.contains(&pointed_file) {
                        return Err(Error::Fs(FsError::Loop(pointed_file)));
                    }
                    visited_symlinks.push(pointed_file.clone());

                    let pointed_path = Path::from_str(&pointed_file).map_err(Error::Path)?;

                    let complete_path = match TryInto::<Path>::try_into(&components) {
                        Ok(remaining_path) => pointed_path.join(&remaining_path),
                        Err(_) => pointed_path,
                    };

                    if complete_path.len() >= PATH_MAX {
                        Err(Error::Fs(FsError::NameTooLong(complete_path.to_string())))
                    } else {
                        path_resolution(fs, &complete_path, current_dir, symlink_resolution, visited_symlinks)
                    }
                },
            }
        }

        path_resolution(self, path, current_dir, symlink_resolution, vec![])
    }

    /// Creates a new file with the given `file_type` at the given `path`.
    ///
    /// # Errors
    ///
    /// Returns a [`Error::Device`] if the device could not be read/written.
    ///
    /// Otherwise, returns a [`Error::Fs`] in any other case.
    fn create_file(
        &mut self,
        path: &Path<'_>,
        file_type: Type,
        permissions: Permissions,
        user_id: Uid,
        group_id: Gid,
    ) -> Result<TypeWithFile<Dir>, Error<Dir::FsError>>
    where
        Self: Sized,
    {
        if path.is_relative() {
            return Err(Error::Path(PathError::AbsolutePathRequired(path.to_string())));
        }

        let Some(parent_dir_path) = path.parent() else { return Err(Error::Fs(FsError::EntryAlreadyExist(path.to_string()))) };
        let parent_dir_file = self.get_file(&parent_dir_path, self.root()?, true)?;
        let mut parent_dir = match parent_dir_file {
            TypeWithFile::Directory(dir) => dir,
            TypeWithFile::Regular(_)
            | TypeWithFile::SymbolicLink(_)
            | TypeWithFile::Fifo(_)
            | TypeWithFile::CharacterDevice(_)
            | TypeWithFile::BlockDevice(_)
            | TypeWithFile::Socket(_) => {
                return Err(Error::Fs(FsError::WrongFileType {
                    expected: Type::Directory,
                    given: parent_dir_file.into(),
                }));
            },
        };

        parent_dir.add_entry(
            // SAFETY: the path is absolute and is not reduced to "/" or to "//"
            unsafe { path.file_name().unwrap_unchecked() },
            file_type,
            permissions,
            user_id,
            group_id,
        )
    }

    /// Removes the file at the given `path`.
    ///
    /// # Errors
    ///
    /// Returns a [`Error::Device`] if the device could not be read/written.
    ///
    /// Otherwise, returns a [`Error::Fs`] in any other case.
    fn remove_file(&mut self, path: Path<'_>) -> Result<(), Error<Dir::FsError>>
    where
        Self: Sized,
    {
        if path.is_relative() {
            return Err(Error::Path(PathError::AbsolutePathRequired(path.to_string())));
        }

        let Some(parent_dir_path) = path.parent() else { return Err(Error::Fs(FsError::EntryAlreadyExist(path.to_string()))) };
        let parent_dir_file = self.get_file(&parent_dir_path, self.root()?, true)?;
        let mut parent_dir = match parent_dir_file {
            TypeWithFile::Directory(dir) => dir,
            TypeWithFile::Regular(_)
            | TypeWithFile::SymbolicLink(_)
            | TypeWithFile::Fifo(_)
            | TypeWithFile::CharacterDevice(_)
            | TypeWithFile::BlockDevice(_)
            | TypeWithFile::Socket(_) => {
                return Err(Error::Fs(FsError::WrongFileType {
                    expected: Type::Directory,
                    given: parent_dir_file.into(),
                }));
            },
        };

        // SAFETY: the path is absolute and is not reduced to "/" or to "//"
        parent_dir.remove_entry(unsafe { path.file_name().unwrap_unchecked() })
    }
}

/// A read-only filesystem.
pub trait ReadOnlyFileSystem<RoDir: ReadOnlyDirectory> {
    /// Returns the root directory of the filesystem.
    ///
    /// # Errors
    ///
    /// Returns a [`Error::Device`] if the device could not be read.
    ///
    /// Otherwise, returns a [`FsError::Implementation`] in any other case.
    fn root(&self) -> Result<RoDir, Error<RoDir::FsError>>;

    /// Returns the double slash root directory of the filesystem.
    ///
    /// If you do not have any idea of what this is, you are probably looking for [`root`](ReadOnlyFileSystem::root).
    ///
    /// See [`DoubleSlashRootDir`](Component::DoubleSlashRootDir) and [`Path`] for more information.
    ///
    /// # Errors
    ///
    /// Returns a [`Error::Device`] if the device could not be read.
    ///
    /// Otherwise, returns a [`FsError::Implementation`] in any other case.
    fn double_slash_root(&self) -> Result<RoDir, Error<RoDir::FsError>>;

    /// Performs a pathname resolution as described in [this POSIX definition](https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/V1_chap04.html#tag_04_16).
    ///
    /// Returns the file of this filesystem corresponding to the given `path`, starting at the `current_dir`.
    ///
    /// `symlink_resolution` indicates whether the function calling this method is required to act on the symbolic link itself, or
    /// certain arguments direct that the function act on the symbolic link itself.
    ///
    /// # Errors
    ///
    /// Returns a [`Error::Device`] if the device could not be read.
    ///
    /// Returns an [`NotFound`](FsError::NotFound) error if the given path does not leed to an existing path.
    ///
    /// Returns an [`NotDir`](FsError::NotDir) error if one of the components of the file is not a directory.
    ///
    /// Returns an [`Loop`](FsError::Loop) error if a loop is found during the symbolic link resolution.
    ///
    /// Returns an [`NameTooLong`](FsError::NameTooLong) error if the complete path contains more than [`PATH_MAX`] characters.
    ///
    /// Returns an [`NoEnt`](FsError::NoEnt) error if an encountered symlink points to a non-existing file.
    ///
    /// Otherwise, returns a [`FsError::Implementation`] in any other case.
    fn get_file(
        &self,
        path: &Path,
        current_dir: RoDir,
        symlink_resolution: bool,
    ) -> Result<ReadOnlyTypeWithFile<RoDir>, Error<RoDir::FsError>>
    where
        Self: Sized,
    {
        /// Auxiliary function used to store the visited symlinks during the pathname resolution to detect loops caused bt symbolic
        /// links.
        fn path_resolution<FSE: core::error::Error, D: ReadOnlyDirectory<FsError = FSE>>(
            fs: &impl ReadOnlyFileSystem<D>,
            path: &Path,
            mut current_dir: D,
            symlink_resolution: bool,
            mut visited_symlinks: Vec<String>,
        ) -> Result<ReadOnlyTypeWithFile<D>, Error<FSE>> {
            let canonical_path = path.canonical();

            if canonical_path.len() > PATH_MAX {
                return Err(Error::Fs(FsError::NameTooLong(canonical_path.to_string())));
            }

            let trailing_blackslash = canonical_path.as_unix_str().has_trailing_backslash();
            let mut symlink_encountered = None;

            let mut components = canonical_path.components();

            for (pos, comp) in components.with_position() {
                match comp {
                    Component::RootDir => {
                        if pos == Position::First || pos == Position::Only {
                            current_dir = fs.root()?;
                        } else {
                            unreachable!("The root directory cannot be encountered during the pathname resolution");
                        }
                    },
                    Component::DoubleSlashRootDir => {
                        if pos == Position::First || pos == Position::Only {
                            current_dir = fs.double_slash_root()?;
                        } else {
                            unreachable!("The double slash root directory cannot be encountered during the pathname resolution");
                        }
                    },
                    Component::CurDir => {},
                    Component::ParentDir => {
                        current_dir = current_dir.parent()?;
                    },
                    Component::Normal(filename) => {
                        let children = current_dir.entries()?;
                        let Some(entry) = children.into_iter().find(|entry| entry.filename == filename).map(|entry| entry.file)
                        else {
                            return Err(Error::Fs(FsError::NotFound(filename.to_string())));
                        };

                        #[allow(clippy::wildcard_enum_match_arm)]
                        match entry {
                            ReadOnlyTypeWithFile::Directory(dir) => {
                                current_dir = dir;
                            },

                            // This case is the symbolic link resolution, which is the one described as **not** being the one
                            // explained in the following paragraph from the POSIX definition of the pathname resolution:
                            //
                            // If a symbolic link is encountered during pathname resolution, the behavior shall depend on whether
                            // the pathname component is at the end of the pathname and on the function
                            // being performed. If all of the following are true, then pathname
                            // resolution is complete:
                            //   1. This is the last pathname component of the pathname.
                            //   2. The pathname has no trailing <slash>.
                            //   3. The function is required to act on the symbolic link itself, or certain arguments direct that
                            //      the function act on the symbolic link itself.
                            ReadOnlyTypeWithFile::SymbolicLink(symlink)
                                if (pos != Position::Last && pos != Position::Only)
                                    || !trailing_blackslash
                                    || !symlink_resolution =>
                            {
                                let pointed_file = symlink.get_pointed_file()?.to_owned();
                                if pointed_file.is_empty() {
                                    return Err(Error::Fs(FsError::NoEnt(filename.to_string())));
                                };

                                symlink_encountered = Some(pointed_file);
                                break;
                            },
                            _ => {
                                return if (pos == Position::Last || pos == Position::Only) && !trailing_blackslash {
                                    Ok(entry)
                                } else {
                                    Err(Error::Fs(FsError::NotDir(filename.to_string())))
                                };
                            },
                        }
                    },
                }
            }

            match symlink_encountered {
                None => Ok(ReadOnlyTypeWithFile::Directory(current_dir)),
                Some(pointed_file) => {
                    if visited_symlinks.contains(&pointed_file) {
                        return Err(Error::Fs(FsError::Loop(pointed_file)));
                    }
                    visited_symlinks.push(pointed_file.clone());

                    let pointed_path = Path::from_str(&pointed_file).map_err(Error::Path)?;

                    let complete_path = match TryInto::<Path>::try_into(&components) {
                        Ok(remaining_path) => pointed_path.join(&remaining_path),
                        Err(_) => pointed_path,
                    };

                    if complete_path.len() >= PATH_MAX {
                        Err(Error::Fs(FsError::NameTooLong(complete_path.to_string())))
                    } else {
                        path_resolution(fs, &complete_path, current_dir, symlink_resolution, visited_symlinks)
                    }
                },
            }
        }

        path_resolution(self, path, current_dir, symlink_resolution, vec![])
    }
}
