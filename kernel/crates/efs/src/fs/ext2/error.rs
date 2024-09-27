//! Errors related to Ext2 manipulation.

use alloc::string::String;

use derive_more::derive::Display;

use super::superblock::EXT2_SIGNATURE;

/// Enumeration of possible errors encountered with Ext2's manipulation.
#[allow(clippy::module_name_repetitions)]
#[derive(Debug, PartialEq, Eq, Display)]
#[display("Ext2 Error: {_variant}")]
pub enum Ext2Error {
    /// A bad file type has been found during the parsing of the inode with the given inode number.
    #[display("Bad File Type: an inode contain the mode {_0}, which does not correspond to a valid file type")]
    BadFileType(u16),

    /// A bad magic number has been found during the superblock parsing.
    ///
    /// See [this table](https://wiki.osdev.org/Ext2#Base_Superblock_Fields) for reference.
    #[display("Bad Magic: {_0} has been found while {EXT2_SIGNATURE} was expected")]
    BadMagic(u16),

    /// A ill-formed C-string has been found during a name parsing.
    #[display("Bad String: a ill-formed C-string has been found")]
    BadString,

    /// Tried to set as free a block already free.
    #[display("Block Already Free: tried to set the {_0} block free while already being free")]
    BlockAlreadyFree(u32),

    /// Tried to set as used a block already in use.
    #[display("Block Already in Use: tried to set the {_0} block in use while already being used")]
    BlockAlreadyInUse(u32),

    /// Tried to write a large file while the filesystem does not have the
    /// [`RequiredFeature`](super::superblock::ReadOnlyFeatures::LARGE_FILE) feature set.
    #[display("File Too Large: Tried to write a large file while the filesystem does not have the LARGE_FILE feature set")]
    FileTooLarge,

    /// Tried to write a [`Gid`](crate::types::Gid) containing a value bigger than [`u16::MAX`] in an inode.
    #[display("GID Too Large: tried to write a GID containing the value {_0}, which cannot be contained on a u16")]
    GidTooLarge(u32),

    /// Tried to set as free an inode already free.
    #[display("Inode Already Free: tried to set the inode {_0} as free but is already")]
    InodeAlreadyFree(u32),

    /// Tried to set as free an inode already free.
    #[display("Inode Already in Use: tried to set the inode {_0} in use while already being used")]
    InodeAlreadyInUse(u32),

    /// Given code does not correspond to a valid file system state.
    ///
    /// See [this table](https://wiki.osdev.org/Ext2#File_System_States) for reference.
    #[display("Invalid State: {_0} has been found while 1 or 2 was expected")]
    InvalidState(u16),

    /// Given code does not correspond to a valid error handling method.
    ///
    /// See [this table](https://wiki.osdev.org/Ext2#Error_Handling_Methods) for reference.
    #[display("Invalid Error Handling Method: {_0} was found while 1, 2 or 3 was expected")]
    InvalidErrorHandlingMethod(u16),

    /// Given code does not correspond to a valid compression algorithm.
    ///
    /// See [this table](https://www.nongnu.org/ext2-doc/ext2.html#s-algo-bitmap) for reference.
    #[display("Invalid Compression Algorithm: {_0} was found while 0, 1, 2, 3 or 4 was expected")]
    InvalidCompressionAlgorithm(u32),

    /// The given name is too long to fit in a directory entry.
    #[display("Name Too Long: {_0} is too long to be written in a directory entry")]
    NameTooLong(String),

    /// Tried to access an extended field in a basic superblock.
    #[display("No Extend Field: tried to access an extended field in a superblock that only contains basic fields")]
    NoExtendedFields,

    /// Tried to access a non-existing block group.
    #[display("Non Existing Block Group: tried to access the {_0} block group which does not exist")]
    NonExistingBlockGroup(u32),

    /// Tried to access a non-existing block.
    #[display("Non Existing Block: tried to access the {_0} block which does not exist")]
    NonExistingBlock(u32),

    /// Tried to access a non-existing inode.
    #[display("Non Existing Inode: tried to access the {_0} inode which does not exist")]
    NonExistingInode(u32),

    /// `NotEnoughFreeBlocks(requested, available)`: Requested more free blocks than currently available.
    #[display("Not Enough Free Blocks: requested {requested} free blocks while only {available} are available")]
    NotEnoughFreeBlocks {
        /// Number of requested blocks.
        requested: u32,

        /// Number of currently available blocks.
        available: u32,
    },

    /// Requested an inode while none is available.
    #[display("Not Enough Inodes: requested an inode but all inodes are in use")]
    NotEnoughInodes,

    /// Tried to access a byte which is out of bounds.
    #[display("Out of Bounds: tried to access the {_0}th byte which is out of bounds")]
    OutOfBounds(i128),

    /// Tried to write a [`Uid`](crate::types::Uid) containing a value bigger than [`u16::MAX`] in an inode.
    #[display("UID Too Large: tried to write a UID containing the value {_0}, which cannot be contained on a u16")]
    UidTooLarge(u32),

    /// An unknown file type has been obtained while parsing a directory entry.
    #[display("Unknown Entry File Type: an unknown file type has been obtained while parsing a directory entry")]
    UnknownEntryFileType,

    /// Filesystem requires a feature that is not supported by this implementation.
    #[display("Unsupported Feature: filesystem requires the {_0} feature which is not supported")]
    UnsupportedFeature(String),
}

impl core::error::Error for Ext2Error {}
