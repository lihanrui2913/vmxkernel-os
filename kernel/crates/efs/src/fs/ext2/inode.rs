//! Interface with ext2's inodes.
//!
//! See the [OSdev wiki](https://wiki.osdev.org/Ext2#Inodes) and the [*The Second Extended Filesystem* book](https://www.nongnu.org/ext2-doc/ext2.html#inode-table) for more information.

use alloc::vec::Vec;
use core::mem::size_of;
use core::slice::from_raw_parts;

use bitflags::bitflags;
use itertools::Itertools;

use super::Ext2;
use super::block_group::BlockGroupDescriptor;
use super::error::Ext2Error;
use super::superblock::{OperatingSystem, Superblock};
use crate::arch::{u32_to_usize, usize_to_u64};
use crate::cache::Cache;
use crate::dev::Device;
use crate::dev::sector::Address;
use crate::error::Error;
use crate::file::Type;
use crate::fs::error::FsError;
use crate::fs::structures::indirection::IndirectedBlocks;
use crate::permissions::Permissions;

/// Number of direct block pointers in an inode.
pub const DIRECT_BLOCK_POINTER_COUNT: u32 = 12;

/// Reserved bad block inode number.
pub const BAD_BLOCKS_INODE: u32 = 1;

/// Reserved root directory inode number.
pub const ROOT_DIRECTORY_INODE: u32 = 2;

/// Reserved ACL index inode number.
pub const ACL_INDEX_INODE: u32 = 3;

/// Reserved ACL index inode number.
pub const ACL_DATA_INODE: u32 = 4;

/// Reserved boot loader inode number.
pub const BOOT_LOADER_INODE: u32 = 5;

/// Reserved undeleted directory inode number.
pub const UNDELETED_DIRECTORY_INODE: u32 = 6;

/// Cache for inodes.
///
/// Stores the couple `((device, inode_number), inode)` for each visited inode.
static INODE_CACHE: Cache<(u32, u32), Inode> = Cache::new();

/// An ext2 inode.
///
/// Each file corresponds to an inode, which contains all the metadata and pointers to the data blocks of the file it represents.
///
/// All the content of a file is located on data blocks, which are common [ext2 blocks](super::block::Block). As a file can grow
/// very large, only [`DIRECT_BLOCK_POINTER_COUNT`] data blocks are directly addressed (by their block numbers). For the further
/// data blocks, an indirection mechanism is created: a special block, called a singly indirected block, will be filled with a table
/// of [`u32`] containing the block numbers of the next data blocks (each [`u32`] corresponds to a block number). This inode
/// contains in the field [`singly_indirect_block_pointer`](struct.Inode.html#structfield.singly_indirect_block_pointer) the block
/// number of the singly indirected block. For the next data blocks, and following this indirection logic, a doubly indirected block
/// and a triply indirected block may also be used. All the indirection logic is deal with the structure [`IndirectedBlocks`].
///
/// Note: **Inode addresses start at 1**.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Inode {
    /// Type and Permissions.
    pub mode: u16,

    /// User ID.
    pub uid: u16,

    /// Lower 32 bits of size in bytes.
    pub size: u32,

    /// Last Access Time (in POSIX time).
    pub atime: u32,

    /// Creation Time (in POSIX time).
    pub ctime: u32,

    /// Last Modification time (in POSIX time).
    pub mtime: u32,

    /// Deletion time (in POSIX time).
    pub dtime: u32,

    /// Group ID.
    pub gid: u16,

    /// Count of hard links (directory entries) to this inode. When this reaches 0, the data blocks are marked as unallocated.
    pub links_count: u16,

    /// Indicates the amount of blocks reserved for the associated file data. This includes both currently in used
    /// and currently reserved blocks in case the file grows in size.
    ///
    ///  Since this value represents 512-byte blocks and not file system blocks, this value should not be directly used as an index
    /// to the `i_block` array. Rather, the maximum index of the `i_block` array should be computed from `i_blocks /
    /// ((1024<<s_log_block_size)/512)`, or once simplified, `i_blocks/(2<<s_log_block_size)`.
    pub blocks: u32,

    /// Flags.
    pub flags: u32,

    /// Operating System Specific value #1.
    pub osd1: u32,

    /// Direct Block Pointers.
    pub direct_block_pointers: [u32; 12],

    /// Singly Indirect Block Pointer (Points to a block that is a list of block pointers to data).
    pub singly_indirect_block_pointer: u32,

    /// Doubly Indirect Block Pointer (Points to a block that is a list of block pointers to Singly Indirect Blocks).
    pub doubly_indirect_block_pointer: u32,

    /// Triply Indirect Block Pointer (Points to a block that is a list of block pointers to Doubly Indirect Blocks).
    pub triply_indirect_block_pointer: u32,

    /// Generation number (Primarily used for NFS).
    pub generation: u32,

    /// In Ext2 version 0, this field is reserved. In version >= 1, Extended attribute block (File ACL).
    pub file_acl: u32,

    /// In Ext2 version 0, this field is reserved. In version >= 1, Upper 32 bits of file size (if feature bit set) if it's a file,
    /// Directory ACL if it's a directory
    pub dir_acl: u32,

    /// Block address of fragment
    pub faddr: u32,

    /// Operating System Specific value #2
    pub osd2: [u8; 12],
}

#[cfg(test)]
impl PartialEq for Inode {
    fn eq(&self, other: &Self) -> bool {
        let self_direct_block_pointers = self.direct_block_pointers;
        let other_direct_block_pointers = other.direct_block_pointers;
        self.mode == other.mode
            && self.uid == other.uid
            && self.size == other.size
            && self.gid == other.gid
            && self.links_count == other.links_count
            && self.blocks == other.blocks
            && self.flags == other.flags
            && self.osd1 == other.osd1
            && self_direct_block_pointers == other_direct_block_pointers
            && self.singly_indirect_block_pointer == other.singly_indirect_block_pointer
            && self.doubly_indirect_block_pointer == other.doubly_indirect_block_pointer
            && self.triply_indirect_block_pointer == other.triply_indirect_block_pointer
            && self.generation == other.generation
            && self.file_acl == other.file_acl
            && self.dir_acl == other.dir_acl
            && self.faddr == other.faddr
            && self.osd2 == other.osd2
    }
}

bitflags! {
    /// Indicators of the inode type and permissions.
    ///
    /// The type indicator occupies the top hex digit (bits 15 to 12).
    ///
    /// The permission indicator occupies the bottom 12 bits.
    #[derive(Debug, Clone, Copy)]
    pub struct TypePermissions: u16 {
        // Types

        /// FIFO
        const FIFO              =   0x1000;

        /// Character device
        const CHARACTER_DEVICE  =   0x2000;

        /// Directory
        const DIRECTORY         =   0x4000;

        /// Block device
        const BLOCK_DEVICE      =   0x6000;

        /// Regular file
        const REGULAR_FILE      =   0x8000;

        /// Symbolic link
        const SYMBOLIC_LINK     =   0xA000;

        /// Unix socket
        const SOCKET            =   0xC000;



        // Permissions

        /// Other - execute permission
        const OTHER_X           =   0x0001;

        /// Other - write permission
        const OTHER_W           =   0x0002;

        /// Other - read permission
        const OTHER_R           =   0x0004;

        /// Group - execute permission
        const GROUP_X           =   0x0008;

        /// Group - write permission
        const GROUP_W           =   0x0010;

        /// Group - read permission
        const GROUP_R           =   0x0020;

        /// User - execute permission
        const USER_X            =   0x0040;

        /// User - write permission
        const USER_W            =   0x0080;

        /// User - read permission
        const USER_R            =   0x0100;

        /// Sticky bit
        const STICKY            =   0x0200;

        /// Set group ID
        const SET_GROUP_ID      =   0x0400;

        /// Set user ID
        const SET_USER_ID       =   0x0800;
    }
}

impl From<TypePermissions> for Permissions {
    fn from(value: TypePermissions) -> Self {
        Self::from_bits_truncate(value.bits())
    }
}

impl From<Type> for TypePermissions {
    fn from(value: Type) -> Self {
        match value {
            Type::Regular => Self::REGULAR_FILE,
            Type::Directory => Self::DIRECTORY,
            Type::SymbolicLink => Self::SYMBOLIC_LINK,
            Type::Fifo => Self::FIFO,
            Type::CharacterDevice => Self::CHARACTER_DEVICE,
            Type::BlockDevice => Self::BLOCK_DEVICE,
            Type::Socket => Self::SOCKET,
        }
    }
}

impl From<Permissions> for TypePermissions {
    fn from(value: Permissions) -> Self {
        Self::from_bits_truncate(value.bits())
    }
}

impl TypePermissions {
    /// Returns the type component of the [`TypePermissions`].
    #[must_use]
    pub const fn file_type(self) -> Self {
        Self::from_bits_truncate((self.bits() >> 12) << 12)
    }
}

bitflags! {
    /// Inode Flags
    #[derive(Debug, Clone, Copy)]
    pub struct Flags: u32 {
        /// Secure deletion (not used)
        const SECURE_DELETION                       =   0x0000_0001;

        /// Keep a copy of data when deleted (not used)
        const DELETION_KEEP_DATA_COPY               =   0x0000_0002;

        /// File compression (not used)
        const FILE_COMPRESSION                      =   0x0000_0004;

        /// Synchronous updates—new data is written immediately to disk
        const SYNCHRONOUS_UPDATES                   =   0x0000_0008;

        /// Immutable file (content cannot be changed)
        const IMMUTABLE_FILE                        =   0x0000_0010;

        /// Append only
        const APPEND_ONLY                           =   0x0000_0020;

        /// File is not included in `dump` command
        const DUMP_EXCLUDED                         =   0x0000_0040;

        /// Last accessed time should not updated
        const LAST_ACCESSED_TIME_UPDATE_DISABLE     =   0x0000_0080;

        /// Hash indexed directory
        const HASHED_INDEXED_DIR                    =   0x0001_0000;

        /// AFS directory
        const AFS_DIR                               =   0x0002_0000;

        /// Journal file data
        const JOURNAL_FILE_DATA                     =   0x0004_0000;

        /// Reserved for ext2 library
        const RESERVED                              =   0x8000_0000;
    }
}

/// OS dependant structure corresponding to [`osd2`](struct.Inode.html#structfield.osd2) field in [`Inode`]
#[derive(Debug, Clone, Copy)]
pub enum Osd2 {
    /// Fields for Hurd systems.
    Hurd {
        /// Fragment number.
        ///
        /// Always 0 GNU HURD since fragments are not supported. Obsolete with Ext4.
        frag: u8,

        /// Fragment size
        ///
        /// Always 0 in GNU HURD since fragments are not supported. Obsolete with Ext4.
        fsize: u8,

        /// High 16bit of the 32bit mode.
        mode_high: u16,

        /// High 16bit of [user id](struct.Inode.html#structfield.uid).
        uid_high: u16,

        /// High 16bit of [group id](struct.Inode.html#structfield.gid).
        gid_high: u16,

        /// Assigned file author.
        ///
        /// If this value is set to -1, the POSIX [user id](struct.Inode.html#structfield.uid) will be used.
        author: u32,
    },

    /// Fields for Linux systems.
    Linux {
        /// Fragment number.
        ///
        /// Always 0 in Linux since fragments are not supported.
        frag: u8,

        /// Fragment size.
        ///
        /// Always 0 in Linux since fragments are not supported.
        fsize: u8,

        /// High 16bit of [user id](struct.Inode.html#structfield.uid).
        uid_high: u16,

        /// High 16bit of [group id](struct.Inode.html#structfield.gid).
        gid_high: u16,
    },

    /// Fields for Masix systems.
    Masix {
        /// Fragment number.
        ///
        /// Always 0 in Masix as framgents are not supported. Obsolete with Ext4.
        frag: u8,

        /// Fragment size.
        ///
        /// Always 0 in Masix as fragments are not supported. Obsolete with Ext4.
        fsize: u8,
    },

    /// Fields for other systems.
    Other([u8; 12]),
}

impl Osd2 {
    /// Get the [`Osd2`] fields from the bytes obtained in the [`Inode`] structure.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 12], os: OperatingSystem) -> Self {
        match os {
            OperatingSystem::Linux => Self::Linux {
                frag: bytes[0],
                fsize: bytes[1],
                uid_high: ((bytes[4] as u16) << 8_usize) + bytes[5] as u16,
                gid_high: ((bytes[6] as u16) << 8_usize) + bytes[7] as u16,
            },
            OperatingSystem::GnuHurd => Self::Hurd {
                frag: bytes[0],
                fsize: bytes[1],
                mode_high: ((bytes[2] as u16) << 8_usize) + bytes[3] as u16,
                uid_high: ((bytes[4] as u16) << 8_usize) + bytes[5] as u16,
                gid_high: ((bytes[6] as u16) << 8_usize) + bytes[7] as u16,
                author: ((bytes[8] as u32) << 24_usize)
                    + ((bytes[9] as u32) << 16_usize)
                    + ((bytes[10] as u32) << 8_usize)
                    + bytes[11] as u32,
            },
            OperatingSystem::Masix => Self::Masix {
                frag: bytes[0],
                fsize: bytes[1],
            },
            OperatingSystem::FreeBSD | OperatingSystem::OtherLites | OperatingSystem::Other(_) => Self::Other(bytes),
        }
    }
}

impl Inode {
    /// Returns the block group of the `n`th inode.
    ///
    /// See the [OSdev wiki](https://wiki.osdev.org/Ext2#Determining_which_Block_Group_contains_an_Inode) for more information.
    #[must_use]
    pub const fn block_group(superblock: &Superblock, n: u32) -> u32 {
        (n - 1) / superblock.base().inodes_per_group
    }

    /// Returns the index of the `n`th inode in its block group.
    ///
    /// See the [OSdev wiki](https://wiki.osdev.org/Ext2#Finding_an_inode_inside_of_a_Block_Group) for more information.
    #[must_use]
    pub const fn group_index(superblock: &Superblock, n: u32) -> u32 {
        (n - 1) % superblock.base().inodes_per_group
    }

    /// Returns the index of the block containing the `n`th inode.
    ///
    /// See the [OSdev wiki](https://wiki.osdev.org/Ext2#Finding_an_inode_inside_of_a_Block_Group) for more information.
    #[must_use]
    pub const fn containing_block(superblock: &Superblock, n: u32) -> u32 {
        n * superblock.inode_size() as u32 / superblock.block_size()
    }

    /// Returns the type and the permissions of this inode.
    #[must_use]
    pub const fn type_permissions(&self) -> TypePermissions {
        TypePermissions::from_bits_truncate(self.mode)
    }

    /// Returns the type of the file pointed by this inode.
    ///
    /// # Errors
    ///
    /// Returns an [`BadFileType`](Ext2Error::BadFileType) if the inode does not contain a valid file type.
    pub const fn file_type(&self) -> Result<Type, Ext2Error> {
        let types_permissions = self.type_permissions();
        if types_permissions.contains(TypePermissions::SYMBOLIC_LINK) {
            Ok(Type::SymbolicLink)
        } else if types_permissions.contains(TypePermissions::REGULAR_FILE) {
            Ok(Type::Regular)
        } else if types_permissions.contains(TypePermissions::DIRECTORY) {
            Ok(Type::Directory)
        } else if types_permissions.contains(TypePermissions::FIFO) {
            Ok(Type::Fifo)
        } else if types_permissions.contains(TypePermissions::CHARACTER_DEVICE) {
            Ok(Type::CharacterDevice)
        } else if types_permissions.contains(TypePermissions::BLOCK_DEVICE) {
            Ok(Type::BlockDevice)
        } else if types_permissions.contains(TypePermissions::SOCKET) {
            Ok(Type::Socket)
        } else {
            Err(Ext2Error::BadFileType(types_permissions.bits()))
        }
    }

    /// Returns the complete size of the data pointed by this inode.
    #[must_use]
    pub const fn data_size(&self) -> u64 {
        // self.size as u64 + ((self.dir_acl as u64) << 32_u64)
        if TypePermissions::contains(&self.type_permissions(), TypePermissions::DIRECTORY) {
            self.size as u64
        } else {
            self.size as u64 + ((self.dir_acl as u64) << 32_u64)
        }
    }

    /// Returns the [`Osd2`] structure given by the [`Inode`] and the [`Superblock`] structures.
    #[must_use]
    pub const fn osd2(&self, superblock: &Superblock) -> Osd2 {
        let os = superblock.creator_operating_system();
        Osd2::from_bytes(self.osd2, os)
    }

    /// Creates a new inode from all the necessary fields.
    #[must_use]
    #[allow(clippy::similar_names)]
    pub const fn create(
        superblock: &Superblock,
        mode: TypePermissions,
        uid: u16,
        gid: u16,
        flags: Flags,
        osd1: u32,
        osd2: [u8; 12],
    ) -> Self {
        Self {
            mode: mode.bits(),
            uid,
            size: 0,
            atime: superblock.base().wtime,
            ctime: superblock.base().wtime,
            mtime: superblock.base().wtime,
            dtime: superblock.base().wtime,
            gid,
            links_count: 1,
            blocks: 0,
            flags: flags.bits(),
            osd1,
            direct_block_pointers: [0_u32; 12],
            singly_indirect_block_pointer: 0,
            doubly_indirect_block_pointer: 0,
            triply_indirect_block_pointer: 0,
            generation: 0,
            file_acl: 0,
            dir_acl: 0,
            faddr: 0,
            osd2,
        }
    }

    /// Returns the starting address of the `n`th inode.
    ///
    /// # Errors
    ///
    /// Returns an [`NonExistingBlockGroup`](Ext2Error::NonExistingBlockGroup) if `n` is greater than the block group count of this
    /// device.
    ///
    /// Otherwise, returns an [`Error::Device`] if the device cannot be read.
    pub fn starting_addr<Dev: Device<u8, Ext2Error>>(fs: &Ext2<Dev>, n: u32) -> Result<Address, Error<Ext2Error>> {
        let base = fs.superblock().base();
        if base.inodes_count < n || n == 0 {
            return Err(Error::Fs(FsError::Implementation(Ext2Error::NonExistingInode(n))));
        };

        let block_group = Self::block_group(fs.superblock(), n);
        let block_group_descriptor = BlockGroupDescriptor::parse(fs, block_group)?;

        let inode_table_starting_block = block_group_descriptor.inode_table;
        let index = Self::group_index(fs.superblock(), n);

        Ok(Address::from(
            inode_table_starting_block * fs.superblock().block_size() + index * u32::from(fs.superblock().inode_size()),
        ))
    }

    /// Parses the `n`th inode from the given device (starting at **1**).
    ///
    /// # Errors
    ///
    /// Returns an [`NonExistingBlockGroup`](Ext2Error::NonExistingBlockGroup) if `n` is greater than the block group count of this
    /// device.
    ///
    /// Returns an [`BadFileType`](Ext2Error::BadFileType) if the inode with the given inode number does not contains a valid file
    /// type.
    ///
    /// Otherwise, returns an [`Error::Device`] if the device cannot be read.
    pub fn parse<Dev: Device<u8, Ext2Error>>(fs: &Ext2<Dev>, n: u32) -> Result<Self, Error<Ext2Error>> {
        if fs.cache
            && let Some(inode) = INODE_CACHE.get_copy(&(fs.device_id, n))
        {
            return Ok(inode);
        }

        let starting_addr = Self::starting_addr(fs, n)?;
        let mut device = fs.device.lock();

        // SAFETY: all the possible failures are catched in the resulting error
        let inode = unsafe { device.read_at::<Self>(starting_addr) }?;

        let inode = match inode.file_type() {
            Ok(_) => inode,
            Err(err) => Err(Error::Fs(FsError::Implementation(err)))?,
        };

        // It's the first time the inode is read.
        if fs.cache {
            INODE_CACHE.insert((fs.device_id, n), inode);
        }

        Ok(inode)
    }

    /// Writes the given `inode` structure at its position.
    ///
    /// # Errors
    ///
    /// Returns an [`Error::Device`] if the device cannot be written.
    ///
    /// # Safety
    ///
    /// The given `inode` must correspond to the given inode number `n`.
    pub(crate) unsafe fn write_on_device<Dev: Device<u8, Ext2Error>>(
        fs: &Ext2<Dev>,
        n: u32,
        inode: Self,
    ) -> Result<(), Error<Ext2Error>> {
        let starting_addr = Self::starting_addr(fs, n)?;
        if fs.cache {
            INODE_CACHE.insert((fs.device_id, n), inode);
        }
        fs.device.lock().write_at(starting_addr, inode)
    }

    /// Returns the complete list of block numbers containing this inode's data (indirect blocks are not considered).
    ///
    /// # Errors
    ///
    /// Returns an [`Error::Device`] if the device cannot be read.
    ///
    /// # Panics
    ///
    /// Panics if the given `superblock` is ill-formed.
    pub fn indirected_blocks<Dev: Device<u8, Ext2Error>>(
        &self,
        fs: &Ext2<Dev>,
    ) -> Result<IndirectedBlocks<DIRECT_BLOCK_POINTER_COUNT>, Error<Ext2Error>> {
        /// Returns the list of block addresses contained in the given indirect block.
        #[allow(clippy::cast_ptr_alignment)]
        fn read_indirect_block<Dev: Device<u8, Ext2Error>>(
            fs: &Ext2<Dev>,
            block_number: u32,
        ) -> Result<Vec<u32>, Error<Ext2Error>> {
            let mut device = fs.device.lock();

            let block_address = Address::from(u32_to_usize(block_number) * u32_to_usize(fs.superblock().block_size()));
            let slice = device.slice(block_address..block_address + u32_to_usize(fs.superblock().block_size()))?;
            let byte_array = slice.as_ref();
            let address_array =
                // SAFETY: casting n `u8` to `u32` with n a multiple of 4 (as the block size is a power of 2, generally above 512)
                unsafe { from_raw_parts::<u32>(byte_array.as_ptr().cast::<u32>(), byte_array.len() / size_of::<u32>()) };
            Ok(address_array.iter().filter(|&block_number| (*block_number != 0)).copied().collect_vec())
        }

        let data_blocks = u32::try_from(1 + (self.data_size().saturating_sub(1)) / u64::from(fs.superblock().block_size()))
            .expect("Ill-formed superblock: there should be at most u32::MAX blocks");
        let mut parsed_data_blocks = 0_u32;
        let blocks_per_indirection = fs.superblock().block_size() / 4;

        let direct_block_pointers = self
            .direct_block_pointers
            .into_iter()
            .filter(|block_number| *block_number != 0)
            .collect_vec();

        parsed_data_blocks += DIRECT_BLOCK_POINTER_COUNT.min(data_blocks);

        let mut indirected_blocks = IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(
            blocks_per_indirection,
            direct_block_pointers,
            (self.singly_indirect_block_pointer, Vec::new()),
            (self.doubly_indirect_block_pointer, Vec::new()),
            (self.triply_indirect_block_pointer, Vec::new()),
        );

        if indirected_blocks.singly_indirected_blocks.0 == 0 || parsed_data_blocks >= data_blocks {
            indirected_blocks.truncate_back_data_blocks(data_blocks);
            return Ok(indirected_blocks);
        }

        indirected_blocks
            .singly_indirected_blocks
            .1
            .append(&mut read_indirect_block(fs, indirected_blocks.singly_indirected_blocks.0)?);
        parsed_data_blocks += blocks_per_indirection;

        if indirected_blocks.doubly_indirected_blocks.0 == 0 || parsed_data_blocks >= data_blocks {
            indirected_blocks.truncate_back_data_blocks(data_blocks);
            return Ok(indirected_blocks);
        }

        let singly_indirect_block_pointers = read_indirect_block(fs, indirected_blocks.doubly_indirected_blocks.0)?;

        for block_pointer in singly_indirect_block_pointers {
            if block_pointer == 0 || parsed_data_blocks >= data_blocks {
                indirected_blocks.truncate_back_data_blocks(data_blocks);
                return Ok(indirected_blocks);
            }

            indirected_blocks
                .doubly_indirected_blocks
                .1
                .push((block_pointer, read_indirect_block(fs, block_pointer)?));
            parsed_data_blocks += blocks_per_indirection;
        }

        if indirected_blocks.triply_indirected_blocks.0 == 0 || parsed_data_blocks >= data_blocks {
            indirected_blocks.truncate_back_data_blocks(data_blocks);
            return Ok(indirected_blocks);
        }

        let triply_indirected_blocks = read_indirect_block(fs, indirected_blocks.triply_indirected_blocks.0)?;

        for block_pointer_pointer in triply_indirected_blocks {
            if block_pointer_pointer == 0 || parsed_data_blocks >= data_blocks {
                indirected_blocks.truncate_back_data_blocks(data_blocks);
                return Ok(indirected_blocks);
            }

            let mut dib = Vec::new();

            let singly_indirect_block_pointers = read_indirect_block(fs, block_pointer_pointer)?;
            parsed_data_blocks += blocks_per_indirection;

            for block_pointer in singly_indirect_block_pointers {
                if block_pointer == 0 || parsed_data_blocks >= data_blocks {
                    indirected_blocks.truncate_back_data_blocks(data_blocks);
                    return Ok(indirected_blocks);
                }

                dib.push((block_pointer, read_indirect_block(fs, block_pointer)?));
            }

            indirected_blocks.triply_indirected_blocks.1.push((block_pointer_pointer, dib));
        }

        indirected_blocks.truncate_back_data_blocks(data_blocks);
        Ok(indirected_blocks)
    }

    /// Reads the content of this inode starting at the given `offset`, returning it in the given `buffer`. Returns the number of
    /// bytes read.
    ///
    /// If the size of the buffer is greater than the inode data size, it will fill the start of the buffer and will leave the end
    /// untouch.
    ///
    /// # Errors
    ///
    /// Returns an [`Error::Device`] if the device cannot be read.
    ///
    /// # Panics
    ///
    /// Panics if the data block starting addresses do not fit on a [`usize`].
    pub fn read_data<Dev: Device<u8, Ext2Error>>(
        &self,
        fs: &Ext2<Dev>,
        buffer: &mut [u8],
        mut offset: u64,
    ) -> Result<usize, Error<Ext2Error>> {
        let indirected_blocks = self.indirected_blocks(fs)?;
        let blocks = indirected_blocks.flatten_data_blocks();

        let mut device = fs.device.lock();
        let buffer_length = buffer.len();

        let mut read_bytes = 0_usize;
        for block_number in blocks {
            if usize_to_u64(read_bytes) == self.data_size() || read_bytes == buffer_length {
                break;
            }

            if offset == 0 {
                let count = u32_to_usize(fs.superblock().block_size()).min(buffer_length - read_bytes);
                let block_addr = Address::from(u32_to_usize(block_number) * u32_to_usize(fs.superblock().block_size()));
                let slice = device.slice(block_addr..block_addr + count)?;

                // SAFETY: buffer contains at least `block_size.min(remaining_bytes_count)` since `remaining_bytes_count <=
                // buffer_length`
                let block_buffer = unsafe { buffer.get_mut(read_bytes..read_bytes + count).unwrap_unchecked() };
                block_buffer.clone_from_slice(slice.as_ref());

                read_bytes += count;
            } else if offset >= u64::from(fs.superblock().block_size()) {
                offset -= u64::from(fs.superblock().block_size());
            } else {
                let data_count = u32_to_usize(fs.superblock().block_size()).min(buffer_length - read_bytes);
                // SAFETY: `offset < superblock.block_size()` and `superblock.block_size()` is generally around few KB, which is
                // fine when `usize > u8`.
                let offset_usize = unsafe { usize::try_from(offset).unwrap_unchecked() };
                match data_count.checked_sub(offset_usize) {
                    None => read_bytes = buffer_length,
                    Some(count) => {
                        let block_addr = Address::from(u32_to_usize(block_number) * u32_to_usize(fs.superblock().block_size()));
                        let slice = device.slice(block_addr + offset_usize..block_addr + offset_usize + count)?;

                        // SAFETY: buffer contains at least `block_size.min(remaining_bytes_count)` since `remaining_bytes_count <=
                        // buffer_length`
                        let block_buffer = unsafe { buffer.get_mut(read_bytes..read_bytes + count).unwrap_unchecked() };
                        block_buffer.clone_from_slice(slice.as_ref());

                        read_bytes += count;
                    },
                }
                offset = 0;
            }
        }

        Ok(read_bytes)
    }

    /// Returns whether this inode is currently free or not from the inode bitmap in which the block resides.
    ///
    /// The `bitmap` argument is usually the result of the method [`get_inode_bitmap`](../struct.Ext2.html#method.get_inode_bitmap).
    #[must_use]
    #[allow(clippy::indexing_slicing)]
    pub const fn is_free(inode_number: u32, superblock: &Superblock, bitmap: &[u8]) -> bool {
        let index = Self::group_index(superblock, inode_number);
        let offset = (inode_number - 1) % 8;
        bitmap[(index / 8) as usize] >> offset & 1 == 0
    }

    /// Returns whether this inode is currently in use or not from the inode bitmap in which the block resides.
    ///
    /// The `bitmap` argument is usually the result of the method [`get_inode_bitmap`](../struct.Ext2.html#method.get_inode_bitmap).
    #[must_use]
    pub const fn is_used(inode_number: u32, superblock: &Superblock, bitmap: &[u8]) -> bool {
        !Self::is_free(inode_number, superblock, bitmap)
    }
}

#[cfg(test)]
mod test {
    use core::mem::size_of;
    use std::fs::File;
    use std::time;

    use crate::dev::Device;
    use crate::fs::ext2::Ext2;
    use crate::fs::ext2::error::Ext2Error;
    use crate::fs::ext2::inode::{Inode, ROOT_DIRECTORY_INODE};
    use crate::tests::{copy_file, new_device_id};

    #[test]
    fn struct_size() {
        assert_eq!(size_of::<Inode>(), 128);
    }

    #[test]
    fn parse_root() {
        let file = copy_file("./tests/fs/ext2/base.ext2").unwrap();
        let fs = Ext2::new(file, new_device_id(), false).unwrap();
        assert!(Inode::parse(&fs, ROOT_DIRECTORY_INODE).is_ok());

        let file = copy_file("./tests/fs/ext2/extended.ext2").unwrap();
        let fs = Ext2::new(file, new_device_id(), false).unwrap();
        assert!(Inode::parse(&fs, ROOT_DIRECTORY_INODE).is_ok());
    }

    #[test]
    fn failed_parse() {
        let file = copy_file("./tests/fs/ext2/base.ext2").unwrap();
        let fs = Ext2::new(file, new_device_id(), false).unwrap();
        assert!(Inode::parse(&fs, 0).is_err());

        let file = copy_file("./tests/fs/ext2/extended.ext2").unwrap();
        let fs = Ext2::new(file, new_device_id(), false).unwrap();
        assert!(Inode::parse(&fs, 0).is_err());
    }

    #[test]
    fn starting_addr() {
        let file = copy_file("./tests/fs/ext2/base.ext2").unwrap();
        let fs = Ext2::new(file, new_device_id(), false).unwrap();

        let root_auto = Inode::parse(&fs, ROOT_DIRECTORY_INODE).unwrap();

        let starting_addr = Inode::starting_addr(&fs, ROOT_DIRECTORY_INODE).unwrap();

        let root_manual =
            unsafe { <File as Device<u8, Ext2Error>>::read_at::<Inode>(&mut fs.device.lock(), starting_addr).unwrap() };

        assert_eq!(root_auto, root_manual);
    }

    #[test]
    fn cache_test() {
        let file = copy_file("./tests/fs/ext2/base.ext2").unwrap();
        let fs = Ext2::new(file, new_device_id(), false).unwrap();

        let start_time = time::Instant::now();
        for _ in 0..100_000 {
            assert!(Inode::parse(&fs, ROOT_DIRECTORY_INODE).is_ok());
        }
        let time_cache_disabled = start_time.elapsed();

        let file = copy_file("./tests/fs/ext2/base.ext2").unwrap();
        let fs = Ext2::new(file, new_device_id(), true).unwrap();
        let start_time = time::Instant::now();
        for _ in 0..100_000 {
            assert!(Inode::parse(&fs, ROOT_DIRECTORY_INODE).is_ok());
        }
        let time_cache_enabled = start_time.elapsed();

        assert!(time_cache_disabled > time_cache_enabled);
    }
}
