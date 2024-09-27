//! # Extended fs
//!
//! An OS and architecture independent implementation of some Unix filesystems in Rust.
//!
//! <div class="warning">
//!
//! This crate is provided as is and do not offer any guaranty. It is still in early development so bugs are excepted to occur. If
//! you find one, please report it at <https://codeberg.org/RatCornu/efs/issues>. In all cases, please do **NOT** use this library for important data, and make sure to backup your data before using it.
//!
//! </div>
//!
//! ## Details
//!
//! This crate provides a general interface to deal with some UNIX filesytems, and adds supports for some of them.
//!
//! Currently, only the [Second Extended Filesystem](https://en.wikipedia.org/wiki/Ext2) (ext2) is supported, but you can implement your own filesystem with this interface.
//!
//! This crate does **NOT** provide a virtual filesystem: you can either make one or use another crate on top on this one.
//!
//! **Every** structure, trait and function in this crate is documented and contains source if needed. If you find something unclear, do not hesitate to create an issue at <https://codeberg.org/RatCornu/efs/issues>.
//!
//! This library sticks as much as possible with the POSIX specification, fully available online on <https://pubs.opengroup.org/onlinepubs/9799919799/>.
//!
//! ### File interfaces
//!
//! * As defined in POSIX, a file can either be a [`Regular`](crate::file::Regular), a [`Directory`](crate::file::Directory), a
//!   [`SymbolicLink`](crate::file::SymbolicLink), a [`Fifo`](crate::file::Fifo), a
//!   [`CharacterDevice`](crate::file::CharacterDevice), a [`BlockDevice`](crate::file::BlockDevice) or a
//!   [`Socket`](crate::file::Socket). Traits are available for each one of them, with basic read and `write` operations. Moreover,
//!   a read-only version of each trait is available.
//!
//! * [`File`](crate::file::File) is the base trait of all other file traits. It provides an interface to retrieve and modify
//!   general attributes of a POSIX file (basically everything returned by the `stat` command on a UNIX OS).
//!
//! * A [`Regular`](crate::file::Regular) (file) is a basic file containing a sequence of bytes, which can be read into a string (or
//!   not, depending on its content). As this file is `no_std` oriented, the use of [`std::io`] is not possible, this is why has to
//!   be manipulated through [`efs::io`](self::io) (you can find [`Read`](crate::io::Read), [`Write`](crate::io::Write) and
//!   [`Seek`](crate::io::Seek) as in [`std::io`]).
//!
//! * A [`Directory`](crate::file::Directory) is a node in the tree-like hierarchy of a filesystem. You can retrieve, add and remove
//!   entries (which are other files).
//!
//! * A [`SymbolicLink`](crate::file::SymbolicLink) is a file pointing an other file. It can be interpreted as the symbolic link or
//!   the pointed file in the [`FileSystem`](crate::fs::FileSystem) trait.
//!
//! * Other file types are defined but cannot be much manipulated as their implementation depends on the virtual file system and on
//!   the OS.
//!
//! ### Filesystem interface
//!
//! All the manipulations needed in a filesystem can be made through the file traits. The [`FileSystem`](crate::fs::FileSystem) is
//! here to provide two things : an entry point to the filesystem with the [`root`](crate::fs::FileSystem::root) method, and
//! high-level functions to make the file manipulations easier.
//!
//! You can read the documentation in the [`fs`] module for more information on [`FileSystem`](crate::fs::FileSystem)s and on how to
//! implement them.
//!
//! ### Paths
//!
//! As the Rust's native [`Path`](std::path::Path) implementation is in [`std::path`], this crates provides an other
//! [`Path`](crate::path::Path) interface. It is based on [`UnixStr`](crate::path::UnixStr), which are the equivalent of
//! [`OsStr`](std::ffi::OsStr) with a guarantee that: it is never empty nor contains the `<NUL>` character ('\0').
//!
//! ### Devices
//!
//! In this crate, a [`Device`](crate::dev::Device) is a sized structure that can be read, written directly at any point.
//!
//! You can read the documentation in the [`dev`] module for more information on [`Device`](dev::Device)s and on how to implement
//! them.
//!
//! ## Usage
//!
//! ### High-level usage
//!
//! You always need to provide two things to use this crate: a filesystem and a device.
//!
//! For the filesystem, you can use the filesystems provided by this crate or make one by yourself (see the [how to implement a
//! filesystem section](#how-to-implement-a-filesystem)). The usage of a filesystem does not depend on whether you are in a `no_std`
//! environment or not.
//!
//! For the devices :
//!
//! * if you are in a `no_std` environment: you can make test with `Vec<u8>`, `&[u8]`, ... if needed, but you will probably have to
//!   provide your own device implementation. See the part on [how to implement a device](#how-to-implement-a-device) if needed.
//!
//! * if you are in a `std` environment: you can use every structure that implements [`std::io::Read`], [`std::io::Write`] and
//!   [`std::io::Seek`] through the use of [`StdIOWrapper`](crate::io::StdIOWrapper). Moreover, you can directly use std's
//!   [`File`](std::fs::File) like this:
//!
//!     ```
//!     use std::fs::File;
//!
//!     use efs::dev::Device;
//!
//!     let file = File::options()
//!         .read(true)
//!         .write(true)
//!         .open("./tests/fs/ext2/io_operations.ext2")
//!         .unwrap();
//!
//!     // `file` is a `Device`
//!     ```
//!
//! ### Concurrency
//!
//! This library do not offer any guaranty for the behaviour of file manipulations when an other program is making `write`
//! operations on the same device at the same time in a general context. Each filesystem implementation documentation contains a
//! paragraph on its handle on concurrency, you can refer to it for more information.
//!
//! In concrete terms, in particular for OS developers, it's your duty, and more precisely the duty of the [virtual filesystem](https://en.wikipedia.org/wiki/Virtual_file_system)
//! to handle the case where two programs tries to modify the same data at the same time.
//!
//! ### Example
//!
//! Here is a complete example of what can be do with the interfaces provided.
//!
//! You can find this test file on [efs's codeberg repo](https://codeberg.org/RatCornu/efs).
//!
//! ```
//! use core::str::FromStr;
//!
//! use efs::file::{Directory, SymbolicLink, Type, TypeWithFile};
//! use efs::fs::FileSystem;
//! use efs::fs::ext2::Ext2Fs;
//! use efs::io::{Read, Write};
//! use efs::path::{Path, UnixStr};
//! use efs::permissions::Permissions;
//! use efs::types::{Gid, Uid};
//!
//! # std::fs::copy(
//! #     "./tests/fs/ext2/io_operations.ext2",
//! #     "./tests/fs/ext2/example.ext2",
//! # )
//! # .unwrap();
//!
//! // `device` now contains a `Device`
//! let device = std::fs::File::options()
//!     .read(true)
//!     .write(true)
//!     .open("./tests/fs/ext2/example.ext2")
//!     .unwrap();
//!
//! let fs = Ext2Fs::new(device, 0, false).unwrap();
//!
//! // `fs` now contains a `FileSystem` with the following structure:
//! // /
//! // ├── bar.txt -> foo.txt
//! // ├── baz.txt
//! // ├── folder
//! // │   ├── ex1.txt
//! // │   └── ex2.txt -> ../foo.txt
//! // ├── foo.txt
//! // └── lost+found
//!
//! /// The root of the filesystem
//! let root = fs.root().unwrap();
//!
//! // We retrieve here `foo.txt` which is a regular file
//! let Some(TypeWithFile::Regular(mut foo_txt)) =
//!     root.entry(UnixStr::new("foo.txt").unwrap()).unwrap()
//! else {
//!     panic!("foo.txt is a regular file in the root folder");
//! };
//!
//! // We read the content of `foo.txt`.
//! assert_eq!(foo_txt.read_all().unwrap(), b"Hello world!\n");
//!
//! // We retrieve here `folder` which is a directory
//! let Some(TypeWithFile::Directory(mut folder)) =
//!     root.entry(UnixStr::new("folder").unwrap()).unwrap()
//! else {
//!     panic!("folder is a directory in the root folder");
//! };
//!
//! // In `folder`, we retrieve `ex1.txt` as `/folder/ex1` points to the same
//! // file as `../folder/ex1.txt` when `/folder` is the current directory.
//! //
//! // Here, it is done by the complete path using the `FileSystem` trait.
//! let Ok(TypeWithFile::Regular(mut ex1_txt)) =
//!     fs.get_file(&Path::from_str("../folder/ex1.txt").unwrap(), folder.clone(), false)
//! else {
//!     panic!("ex1.txt is a regular file at /folder/ex1.txt");
//! };
//!
//! // We read the content of `foo.txt`.
//! ex1_txt.write_all(b"Hello earth!\n").unwrap();
//!
//! // We can also retrieve/create/delete a subentry with the `Directory`
//! // trait.
//! let TypeWithFile::SymbolicLink(mut boo) = folder
//!     .add_entry(
//!         UnixStr::new("boo.txt").unwrap(),
//!         Type::SymbolicLink,
//!         Permissions::from_bits_retain(0o777),
//!         Uid(0),
//!         Gid(0),
//!     )
//!     .unwrap()
//! else {
//!     panic!("Could not create a symbolic link");
//! };
//!
//! // We set the pointed file of the newly created `/folder/boo.txt` to
//! // `../baz.txt`.
//! boo.set_pointed_file("../baz.txt").unwrap();
//!
//! // We ensure now that if we read `/folder/boo.txt` while following the
//! // symbolic links we get the content of `/baz.txt`.
//! let TypeWithFile::Regular(mut baz_txt) =
//!     fs.get_file(&Path::from_str("/folder/boo.txt").unwrap(), root, true).unwrap()
//! else {
//!     panic!("Could not retrieve baz.txt from boo.txt");
//! };
//! assert_eq!(ex1_txt.read_all().unwrap(), baz_txt.read_all().unwrap());
//!
//! // Here is the state of the filesystem at the end of this example:
//! // /
//! // ├── bar.txt -> foo.txt
//! // ├── baz.txt
//! // ├── folder
//! // │   ├── boo.txt -> ../baz.txt
//! // │   ├── ex1.txt
//! // │   └── ex2.txt -> ../foo.txt
//! // ├── foo.txt
//! // └── lost+found
//!
//! # std::fs::remove_file("./tests/fs/ext2/example.ext2").unwrap();
//! ```

#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(
    clippy::absolute_paths,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::else_if_without_else,
    clippy::exhaustive_enums,
    clippy::exhaustive_structs,
    clippy::expect_used,
    clippy::implicit_return,
    clippy::integer_division,
    clippy::missing_trait_methods,
    clippy::mod_module_files,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::pattern_type_mismatch,
    clippy::pub_with_shorthand,
    clippy::question_mark_used,
    clippy::separated_literal_suffix,
    clippy::shadow_reuse,
    clippy::shadow_unrelated,
    clippy::todo,
    clippy::unreachable,
    clippy::use_debug,
    clippy::unwrap_in_result,
    clippy::wildcard_in_or_patterns
)]
#![cfg_attr(
    test,
    allow(
        clippy::assertions_on_result_states,
        clippy::collection_is_never_read,
        clippy::enum_glob_use,
        clippy::indexing_slicing,
        clippy::non_ascii_literal,
        clippy::too_many_lines,
        clippy::undocumented_unsafe_blocks,
        clippy::unwrap_used,
        clippy::wildcard_imports
    )
)]
#![feature(exact_size_is_empty)]
#![feature(let_chains)]
#![feature(never_type)]
#![feature(step_trait)]

extern crate alloc;
extern crate core;
#[cfg(feature = "std")]
extern crate std;

pub mod arch;
pub mod cache;
pub mod celled;
pub mod dev;
pub mod error;
pub mod file;
pub mod fs;
pub mod io;
pub mod path;
pub mod permissions;
#[cfg(test)]
pub(crate) mod tests;
pub mod types;
