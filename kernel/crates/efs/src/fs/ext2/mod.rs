//! # ext2
//!
//! Implementation of the Second Extended Filesystem (ext2fs) filesystem.
//!
//! See [its Wikipedia page](https://en.wikipedia.org/wiki/Ext2), [its kernel.org page](https://www.kernel.org/doc/html/latest/filesystems/ext2.html), [its OSDev page](https://wiki.osdev.org/Ext2), and the [*The Second Extended Filesystem* book](https://www.nongnu.org/ext2-doc/ext2.html) for more information.
//!
//! ## Description
//!
//! The ext2 filesystem structure is such like this:
//!
//! ```txt
//! +-----------------------------------------------------------+
//! |                                                           |
//! +-----------------------------------------------------------+
//! |                        Superblock                         |
//! | (Filesystem metadata: size, number of blocks, inodes,     |
//! |  free space, etc.)                                        |
//! +-----------------------------------------------------------+
//! |                     Group Descriptors                     |
//! | (Pointers to block and inode bitmaps, inode tables,       |
//! |  free block counts, etc. for each block group)            |
//! +-----------------------------+-----------------------------+
//! |        Block Group 1        |        Block Group 2      | |
//! | +-------------------------+ | +-------------------------+ |
//! | | Block Bitmap            | | | Block Bitmap            | |
//! | +-------------------------+ | +-------------------------+ |
//! | | Inode Bitmap            | | | Inode Bitmap            | |
//! | +-------------------------+ | +-------------------------+ |
//! | | Inode Table             | | | Inode Table             | |
//! | +-------------------------+ | +-------------------------+ |
//! | | Data Blocks             | | | Data Blocks             | |
//! | | (Files, directories,    | | | (Files, directories,    | |
//! | |  etc.)                  | | |  etc.)                  | |
//! | +-------------------------+ | +-------------------------+ |
//! +-----------------------------+-----------------------------+
//!                             ...
//! +-----------------------------+-----------------------------+
//! |        Block Group N - 1    |        Block Group N      | |
//! | +-------------------------+ | +-------------------------+ |
//! | | Block Bitmap            | | | Block Bitmap            | |
//! | +-------------------------+ | +-------------------------+ |
//! | | Inode Bitmap            | | | Inode Bitmap            | |
//! | +-------------------------+ | +-------------------------+ |
//! | | Inode Table             | | | Inode Table             | |
//! | +-------------------------+ | +-------------------------+ |
//! | | Data Blocks             | | | Data Blocks             | |
//! | | (Files, directories,    | | | (Files, directories,    | |
//! | |  etc.)                  | | |  etc.)                  | |
//! | +-------------------------+ | +-------------------------+ |
//! +-----------------------------+-----------------------------+
//! ```
//!
//! Important things to know about ext2:
//!
//! - The [`Device`] is splitted in contiguous [`Block`](block::Block)s that have all the same size in bytes. This is **NOT** the
//!   block as in block device, here "block" always refers to ext2's blocks. They start at 0, so the `n`th block will start at the
//!   adress `n * block_size`.
//!
//! - The [superblock](Superblock) contains all the important filesystem metadata, such as the total number of blocks, the number of
//!   free blocks, the state of the filesystem, the supported features, etc.
//!
//! - The data blocks are splitted in block groups containing the same amount of blocks. Each block group is descriped by a [block
//!   group descriptor](BlockGroupDescriptor) containing all the relevant metadata about its block group.
//!
//! - Each file is described by an [inode](Inode) containing all the metadata of the file and pointers to the data blocks. See the
//!   documentation of the [`Inode`] structure for more information.
//!
//! - The block and inode bitmaps keep track of which blocks and inodes are currently used or not. This is useful to find empty
//!   space when writting in a file or allocating a new file.
//!
//! ## Concurrency
//!
//! When instancing a new instance of [`Ext2Fs`], the principal structure to manipulate an ext2 filesystem, you can choose whether
//! to enable caches or not.
//!
//! - When the cache is not enabled, all the structures will be read whenever needed to make sure no change has been made by an
//!   external program. Only the [superblock](Superblock) will be kept in memory because of its high usage: it will only be updated
//!   before each write operation.
//!
//! - When the cache is enabled the [superblock](Superblock), the [inodes](Inode), the [block group
//!   descriptors](BlockGroupDescriptor) and the [directories](Directory) will be cached so that the program does not need to read
//!   the current state of the filesystem to know the content of those structures.
//!
//! Enabling the cache will consume more memory but can greatly increase speed when dealing with a high number of requests.
//!
//! The cache **CANNOT** be used if an external program is writing on this filesystem at the same time. When using the cache, you
//! have to make sure that all the write operations are made by this library.

use alloc::borrow::ToOwned;
use alloc::vec;
use alloc::vec::Vec;
use core::mem::size_of;

use derive_more::derive::{Deref, DerefMut};
use file::{BlockDevice, CharacterDevice, Fifo, Socket};

use self::block_group::BlockGroupDescriptor;
use self::error::Ext2Error;
use self::file::{Directory, Regular, SYMBOLIC_LINK_INODE_STORE_LIMIT, SymbolicLink};
use self::inode::{Flags, Inode, ROOT_DIRECTORY_INODE, TypePermissions};
use self::superblock::{ReadOnlyFeatures, RequiredFeatures, SUPERBLOCK_START_BYTE, Superblock};
use super::FileSystem;
use super::structures::bitmap::Bitmap;
use crate::arch::{u32_to_usize, usize_to_u64};
use crate::celled::Celled;
use crate::dev::Device;
use crate::dev::sector::Address;
use crate::error::Error;
use crate::file::{Type, TypeWithFile};
use crate::fs::error::FsError;

pub mod block;
pub mod block_group;
pub mod directory;
pub mod error;
pub mod file;
pub mod inode;
pub mod superblock;

#[cfg(not(any(target_pointer_width = "32", target_pointer_width = "64")))]
compile_error!("The ext2 feature cannot be enabled on architectures with pointer width other than 32 or 64 bits.");

/// Type alias to reduce complexity in functions' types.
#[allow(clippy::module_name_repetitions)]
pub type Ext2TypeWithFile<Dev> = TypeWithFile<Directory<Dev>>;

/// Contains all the supported options of the filesystem.
#[derive(Debug, Clone)]
struct Options {
    /// Whether the file system can handle a 64-bit file size.
    large_files: bool,

    /// Whether directory entries contain a type field.
    file_type: bool,
}

/// Interface to manipulate devices containing an ext2 filesystem.
#[derive(Debug, Clone)]
pub struct Ext2<Dev: Device<u8, Ext2Error>> {
    /// Device number of the device containing the ext2 filesystem.
    device_id: u32,

    /// Device containing the ext2 filesystem.
    device: Celled<Dev>,

    /// Superblock of the filesystem.
    superblock: Superblock,

    /// Options of the filesystem.
    options: Options,

    /// Whether to enable cache for this filesystem.
    cache: bool,
}

impl<Dev: Device<u8, Ext2Error>> Ext2<Dev> {
    /// Creates a new [`Ext2`] object from the given device that should contain an ext2 filesystem and a given device ID.
    ///
    /// # Errors
    ///
    /// Returns an [`Error::Device`] if the device cannot be read.
    ///
    /// Returns [`Ext2Error::BadMagic`] if the magic number found in the superblock is not equal to
    /// [`EXT2_SIGNATURE`](superblock::EXT2_SIGNATURE).
    pub fn new(device: Dev, device_id: u32, cache: bool) -> Result<Self, Error<Ext2Error>> {
        let celled_device = Celled::new(device);
        Self::new_celled(celled_device, device_id, cache)
    }

    /// Creates a new [`Ext2`] object from the given celled device that should contain an ext2 filesystem and a given device ID.
    ///
    /// # Errors
    ///
    /// Returns an [`Error::Device`] if the device cannot be read.
    ///
    /// Returns [`Ext2Error::UnsupportedFeature`] if the filesystem requires a feature that is unsupported by this implementation.
    ///
    /// Returns [`Ext2Error::BadMagic`] if the magic number found in the superblock is not equal to
    /// [`EXT2_SIGNATURE`](superblock::EXT2_SIGNATURE).
    pub fn new_celled(celled_device: Celled<Dev>, device_id: u32, cache: bool) -> Result<Self, Error<Ext2Error>> {
        let superblock = Superblock::parse(&celled_device)?;

        if let Ok(required_features) = superblock.required_features() {
            if required_features.contains(RequiredFeatures::COMPRESSION) {
                return Err(Error::Fs(FsError::Implementation(Ext2Error::UnsupportedFeature("compression".to_owned()))));
            } else if required_features.contains(RequiredFeatures::JOURNAL_DEV) {
                return Err(Error::Fs(FsError::Implementation(Ext2Error::UnsupportedFeature("journal_dev".to_owned()))));
            } else if required_features.contains(RequiredFeatures::META_BG) {
                return Err(Error::Fs(FsError::Implementation(Ext2Error::UnsupportedFeature("meta_bg".to_owned()))));
            } else if required_features.contains(RequiredFeatures::RECOVER) {
                return Err(Error::Fs(FsError::Implementation(Ext2Error::UnsupportedFeature("recover".to_owned()))));
            }
        }

        let options = Options {
            large_files: superblock
                .read_only_features()
                .is_ok_and(|read_only_features| read_only_features.contains(ReadOnlyFeatures::LARGE_FILE)),
            file_type: superblock
                .read_only_features()
                .is_ok_and(|read_only_features| read_only_features.contains(ReadOnlyFeatures::LARGE_FILE)),
        };

        Ok(Self {
            device_id,
            device: celled_device,
            superblock,
            options,
            cache,
        })
    }

    /// Returns the [`Superblock`] of this filesystem.
    #[must_use]
    pub const fn superblock(&self) -> &Superblock {
        &self.superblock
    }

    /// Returns the [`Options`] of this filesystem.
    #[must_use]
    const fn options(&self) -> &Options {
        &self.options
    }

    /// Returns the [`Inode`] with the given number.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Inode::parse`].
    pub fn inode(&self, inode_number: u32) -> Result<Inode, Error<Ext2Error>> {
        Inode::parse(self, inode_number)
    }

    /// Updates the inner [`Superblock`].
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Superblock::parse`].
    fn update_inner_superblock(&mut self) -> Result<(), Error<Ext2Error>> {
        self.superblock = Superblock::parse(&self.device)?;
        Ok(())
    }

    /// Sets the device's superblock to the given object.
    ///
    /// # Errors
    ///
    ///
    /// Returns an [`Error::Device`] if the device cannot be written.
    ///
    /// # Safety
    ///
    /// Must ensure that the given superblock is coherent with the current state of the filesystem.
    unsafe fn set_superblock(&mut self, superblock: &Superblock) -> Result<(), Error<Ext2Error>> {
        let mut device = self.device.lock();
        device.write_at(Address::new(SUPERBLOCK_START_BYTE), *superblock.base())?;

        if let Some(extended) = superblock.extended_fields() {
            device.write_at(Address::new(SUPERBLOCK_START_BYTE + size_of::<superblock::Base>()), *extended)?;
        }

        drop(device);

        self.update_inner_superblock()
    }

    /// Sets the counter of free blocks in the superblock.
    ///
    /// # Errors
    ///
    /// Returns an [`Error::Device`] if the device cannot be written.
    ///
    /// # Safety
    ///
    /// Must ensure that the given `value` corresponds to the real count of free blocks in the filesystem.
    unsafe fn set_free_blocks(&mut self, value: u32) -> Result<(), Error<Ext2Error>> {
        let mut superblock = self.superblock().clone();
        superblock.base_mut().free_blocks_count = value;

        self.set_superblock(&superblock)
    }

    /// Returns the block bitmap for the given block group.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`BlockGroupDescriptor::parse`](BlockGroupDescriptor::parse).
    pub fn get_block_bitmap(&self, block_group_number: u32) -> Result<Bitmap<u8, Ext2Error, Dev>, Error<Ext2Error>> {
        let superblock = self.superblock();

        let block_group_descriptor = BlockGroupDescriptor::parse(self, block_group_number)?;
        let starting_addr = Address::new(u32_to_usize(block_group_descriptor.block_bitmap) * u32_to_usize(superblock.block_size()));
        let length = u32_to_usize(superblock.base().blocks_per_group / 8);

        Bitmap::new(self.device.clone(), starting_addr, length)
    }

    /// Returns a [`Vec`] containing the block numbers of `n` free blocks.
    ///
    /// Looks for free blocks starting at the block group `start_block_group`.
    ///
    /// # Errors
    ///
    /// Returns an [`NotEnoughFreeBlocks`](Ext2Error::NotEnoughFreeBlocks) error if requested more free blocks than available.
    ///
    /// Returns an [`Error::Device`] if the device cannot be read.
    pub fn free_blocks_offset(&self, n: u32, start_block_group: u32) -> Result<Vec<u32>, Error<Ext2Error>> {
        if n == 0 {
            return Ok(vec![]);
        }

        if n > self.superblock().base().free_blocks_count {
            return Err(Error::Fs(FsError::Implementation(Ext2Error::NotEnoughFreeBlocks {
                requested: n,
                available: self.superblock().base().free_blocks_count,
            })));
        }

        let total_block_group_count = self.superblock().base().block_group_count();

        let mut free_blocks = Vec::new();

        for mut block_group_count in 0_u32..total_block_group_count {
            block_group_count = (block_group_count + start_block_group) % total_block_group_count;

            let block_group_descriptor = BlockGroupDescriptor::parse(self, block_group_count)?;
            if block_group_descriptor.free_blocks_count > 0 {
                let bitmap = self.get_block_bitmap(block_group_count)?;
                let group_free_block_index = bitmap.find_n_unset_bits(u32_to_usize(n));

                for (index, byte) in group_free_block_index {
                    // SAFETY: a block size is usually at most thousands of bytes, which is smaller than `u32::MAX`
                    let index = unsafe { u32::try_from(index).unwrap_unchecked() };

                    for bit in 0_u32..8 {
                        if (byte >> bit) & 1 == 0 {
                            free_blocks.push(
                                block_group_count * self.superblock().base().blocks_per_group
                                    + index * 8
                                    + bit
                                    + self.superblock().base().first_data_block,
                            );

                            if usize_to_u64(free_blocks.len()) == u64::from(n) {
                                return Ok(free_blocks);
                            }
                        }
                    }
                }
            }
        }

        // SAFETY: free_blocks.len() is smaller than n  which is a u32
        Err(Error::Fs(FsError::Implementation(Ext2Error::NotEnoughFreeBlocks {
            requested: n,
            available: unsafe { free_blocks.len().try_into().unwrap_unchecked() },
        })))
    }

    /// Returns a [`Vec`] containing the block numbers of `n` free blocks.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`free_blocks_offset`](Ext2::free_blocks_offset).
    pub fn free_blocks(&self, n: u32) -> Result<Vec<u32>, Error<Ext2Error>> {
        self.free_blocks_offset(n, 0)
    }

    /// Sets all the given `blocks` as `usage` in their bitmap, and updates the block group descriptors and the superblock
    /// accordingly.
    ///
    /// # Errors
    ///
    /// Returns an [`Error::Device`] if the device cannot be read/written.
    ///
    /// Returns an [`BlockAlreadyInUse`](Ext2Error::BlockAlreadyInUse) error if it tries to allocate a block that was already in
    /// use.
    ///
    /// Returns an [`BlockAlreadyFree`](Ext2Error::BlockAlreadyFree) error if it tries to deallocate a block that was already free.
    ///
    /// Returns an [`NonExistingBlock`](Ext2Error::NonExistingBlock) error if a given block does not exist.
    ///
    /// Otherwise, returns the same errors as [`get_block_bitmap`](Ext2::get_block_bitmap).
    fn locate_blocks(&mut self, blocks: &[u32], usage: bool) -> Result<(), Error<Ext2Error>> {
        /// Updates the block group bitmap and the free block count in the descriptor.
        ///
        /// # Errors
        ///
        /// Returns an [`Error::Device`] if the device cannot be read.
        ///
        /// # Safety
        ///
        /// Must ensure that the `number_blocks_changed_in_group` is coherent with the given `bitmap`.
        unsafe fn update_block_group<Dev: Device<u8, Ext2Error>>(
            ext2: &Ext2<Dev>,
            block_group_number: u32,
            number_blocks_changed_in_group: u16,
            bitmap: &mut Bitmap<u8, Ext2Error, Dev>,
            usage: bool,
        ) -> Result<(), Error<Ext2Error>> {
            bitmap.write_back()?;

            let mut new_block_group_descriptor = BlockGroupDescriptor::parse(ext2, block_group_number)?;

            if usage {
                new_block_group_descriptor.free_blocks_count -= number_blocks_changed_in_group;
            } else {
                new_block_group_descriptor.free_blocks_count += number_blocks_changed_in_group;
            }
            BlockGroupDescriptor::write_on_device(ext2, block_group_number, new_block_group_descriptor)
        }

        if !self.cache {
            self.update_inner_superblock()?;
        }

        let block_opt = blocks.first();

        let mut number_blocks_changed_in_group = 0_u16;
        // SAFETY: the total number of blocks in the filesystem is a u32
        let total_number_blocks_changed = unsafe { u32::try_from(blocks.len()).unwrap_unchecked() };

        if let Some(block) = block_opt {
            let mut block_group_number = self.superblock().block_group(*block);
            let mut bitmap = self.get_block_bitmap(block_group_number)?;

            for block in blocks {
                if self.superblock().block_group(*block) != block_group_number {
                    // SAFETY: the state of the filesystem stays coherent within this function
                    unsafe {
                        update_block_group(self, block_group_number, number_blocks_changed_in_group, &mut bitmap, usage)?;
                    };

                    number_blocks_changed_in_group = 0;

                    block_group_number = self.superblock().block_group(*block);
                    bitmap = self.get_block_bitmap(block_group_number)?;
                }

                number_blocks_changed_in_group += 1;

                let group_index = self.superblock().group_index(*block);
                let bitmap_index = group_index / 8;
                let bitmap_offset = group_index % 8;

                let Some(byte) = bitmap.get_mut(u32_to_usize(bitmap_index)) else {
                    return Err(Error::Fs(FsError::Implementation(Ext2Error::NonExistingBlock(*block))));
                };

                if usage && *byte >> bitmap_offset & 1 == 1 {
                    return Err(Error::Fs(FsError::Implementation(Ext2Error::BlockAlreadyInUse(*block))));
                } else if !usage && *byte >> bitmap_offset & 1 == 0 {
                    return Err(Error::Fs(FsError::Implementation(Ext2Error::BlockAlreadyFree(*block))));
                }

                *byte ^= 1 << bitmap_offset;
            }

            // SAFETY: the state of the filesystem stays coherent within this function
            unsafe {
                update_block_group(self, block_group_number, number_blocks_changed_in_group, &mut bitmap, usage)?;
            };
        }

        if usage {
            // SAFETY: the total number of free blocks is the one before minus the total number of allocated blocks
            unsafe {
                self.set_free_blocks(self.superblock().base().free_blocks_count - total_number_blocks_changed)?;
            };
        } else {
            // SAFETY: the total number of free blocks is the one before plus the total number of deallocated blocks
            unsafe {
                self.set_free_blocks(self.superblock().base().free_blocks_count + total_number_blocks_changed)?;
            };
        }

        Ok(())
    }

    /// Sets all the given `blocs` as "used".
    ///
    /// # Errors
    ///
    /// Returns an [`Error::Device`] if the device cannot be read/written.
    ///
    /// Returns an [`BlockAlreadyInUse`](Ext2Error::BlockAlreadyInUse) error if the given block was already in use.
    ///
    /// Otherwise, returns the same errors as [`get_block_bitmap`](Ext2::get_block_bitmap).
    pub fn allocate_blocks(&mut self, blocks: &[u32]) -> Result<(), Error<Ext2Error>> {
        self.locate_blocks(blocks, true)
    }

    /// Sets all the given `blocs` as "free".
    ///
    /// # Errors
    ///
    /// Returns an [`Error::Device`] if the device cannot be read/written.
    ///
    /// Returns an [`BlockAlreadyFree`](Ext2Error::BlockAlreadyFree) error if the given block was already in use.
    ///
    /// Otherwise, returns the same errors as [`get_block_bitmap`](Ext2::get_block_bitmap).
    pub fn deallocate_blocks(&mut self, blocks: &[u32]) -> Result<(), Error<Ext2Error>> {
        self.locate_blocks(blocks, false)
    }

    /// Returns the inode bitmap for the given block group.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`BlockGroupDescriptor::parse`](block_group/struct.BlockGroupDescriptor.html#method.parse).
    pub fn get_inode_bitmap(&self, block_group_number: u32) -> Result<Bitmap<u8, Ext2Error, Dev>, Error<Ext2Error>> {
        let superblock = self.superblock();

        let block_group_descriptor = BlockGroupDescriptor::parse(self, block_group_number)?;
        let starting_addr = Address::new(u32_to_usize(block_group_descriptor.inode_bitmap) * u32_to_usize(superblock.block_size()));
        let length = u32_to_usize(superblock.base().inodes_per_group / 8);

        Bitmap::new(self.device.clone(), starting_addr, length)
    }

    /// Retuns the number of the first unused inode.
    ///
    /// # Errors
    ///
    /// Returns a [`NotEnoughInodes`](Ext2Error::NotEnoughInodes) if no inode is currently available.
    ///
    /// Returns an [`Error::Device`] if the device cannot be read/written.
    pub fn free_inode(&mut self) -> Result<u32, Error<Ext2Error>> {
        if !self.cache {
            self.update_inner_superblock()?;
        }

        for block_group_number in 0..self.superblock().block_group_count() {
            let inode_bitmap = self.get_inode_bitmap(block_group_number)?;
            if let Some(index) = inode_bitmap.iter().position(|byte| *byte != u8::MAX) {
                // SAFETY: the index has been given by the function `position`
                let byte = *unsafe { inode_bitmap.get_unchecked(index) };
                for bit in 0_u32..8 {
                    if (byte >> bit) & 1 == 0 {
                        // SAFETY: a block size is usually at most thousands of bytes, which is smaller than `u32::MAX`
                        let index = unsafe { u32::try_from(index).unwrap_unchecked() };

                        return Ok(block_group_number * self.superblock().base().inodes_per_group + 8 * index + bit + 1);
                    }
                }
            }
        }

        Err(Error::Fs(FsError::Implementation(Ext2Error::NotEnoughInodes)))
    }

    /// Sets the given `inode` as `usage` in their bitmap, and updates the block group descriptors and the superblock
    /// accordingly.
    ///
    /// # Errors
    ///
    /// Returns a [`InodeAlreadyFree`](Ext2Error::InodeAlreadyFree) if the given inode is already free.
    ///
    /// Returns a [`InodeAlreadyInUse`](Ext2Error::InodeAlreadyInUse) if the given inode is already in use.
    ///
    /// Returns a [`NotEnoughInodes`](Ext2Error::NotEnoughInodes) if no inode is currently available.
    ///
    /// Returns an [`Error::Device`] if the device cannot be read/written.
    ///
    /// # Panics
    ///
    /// Panics if the inode starting byte on the device does not fit on a [`usize`].
    ///
    /// More pratically, it could be an issue if you're using a device containing more than 2^35 bytes ~ 34.5 GB.
    fn locate_inode(&mut self, inode_number: u32, usage: bool) -> Result<(), Error<Ext2Error>> {
        let mut superblock = self.superblock().clone();
        if usage {
            superblock.base_mut().free_inodes_count -= 1;
        } else {
            superblock.base_mut().free_inodes_count += 1;
        }

        // SAFETY: one inode has been allocated
        unsafe {
            self.set_superblock(&superblock)?;
        };

        let block_group_number = Inode::block_group(&superblock, inode_number);
        let block_group_descriptor = BlockGroupDescriptor::parse(self, block_group_number)?;

        let mut new_block_group_descriptor = block_group_descriptor;

        if usage {
            new_block_group_descriptor.free_inodes_count -= 1;
        } else {
            new_block_group_descriptor.free_inodes_count += 1;
        }

        // SAFETY: the starting address is the one of the block group descriptor
        unsafe {
            BlockGroupDescriptor::write_on_device(self, block_group_number, new_block_group_descriptor)?;
        };

        let bitmap_starting_byte = u64::from(block_group_descriptor.inode_bitmap) * u64::from(superblock.block_size());

        let inode_index = u64::from(Inode::group_index(&superblock, inode_number)) / 8;
        // SAFETY: 0 <= inode_offset < 8
        let inode_offset = unsafe { u8::try_from((inode_number - 1) % 8).unwrap_unchecked() };
        let bitmap_inode_address = Address::new(
            usize::try_from(bitmap_starting_byte + inode_index).expect("Could not fit the starting byte of the inode on a usize"),
        );

        let mut device = self.device.lock();

        #[allow(clippy::range_plus_one)]
        let mut slice = device.slice(bitmap_inode_address..bitmap_inode_address + 1)?;

        // SAFETY: it is ensure that `slice` contains exactly 1 element
        let byte = unsafe { slice.get_mut(0).unwrap_unchecked() };

        if usage && *byte >> inode_offset & 1 == 1 {
            return Err(Error::Fs(FsError::Implementation(Ext2Error::InodeAlreadyInUse(inode_number))));
        } else if !usage && *byte >> inode_offset & 1 == 0 {
            return Err(Error::Fs(FsError::Implementation(Ext2Error::InodeAlreadyFree(inode_number))));
        }

        *byte ^= 1 << inode_offset;

        let commit = slice.commit();
        device.commit(commit)
    }

    /// Writes an empty inode, sets the usage of this inode as `true` and returns the inode number.
    ///
    /// # Errors
    ///
    /// Returns a [`InodeAlreadyInUse`](Ext2Error::InodeAlreadyInUse) if the given inode is already in use.
    ///
    /// Returns an [`Error::Device`] if the device cannot be read/written.
    ///
    /// # Panics
    ///
    /// Panics if the inode starting byte on the device does not fit on a [`usize`].
    ///
    /// More pratically, it could be an issue if you're using a device containing more than 2^35 bytes ~ 34.5 GB.
    #[allow(clippy::similar_names)]
    #[allow(clippy::too_many_arguments)]
    pub fn allocate_inode(
        &mut self,
        inode_number: u32,
        mode: TypePermissions,
        uid: u16,
        gid: u16,
        flags: Flags,
        osd1: u32,
        osd2: [u8; 12],
    ) -> Result<(), Error<Ext2Error>> {
        self.locate_inode(inode_number, true)?;
        let inode = Inode::create(self.superblock(), mode, uid, gid, flags, osd1, osd2);

        // SAFETY: the starting address is the one for an inode
        unsafe { Inode::write_on_device(self, inode_number, inode) }
    }

    /// Decreases the hard link count of this inode by 1. If the hard link count reaches 0, sets the usage of this inode as `false`
    /// and deallocates every block used by this inode.
    ///
    /// # Errors
    ///
    /// Returns a [`InodeAlreadyFree`](Ext2Error::InodeAlreadyFree) if the given inode is already free.
    ///
    /// Returns an [`Error::Device`] if the device cannot be read/written.
    ///
    /// # Panics
    ///
    /// Panics for the same reasons as [`allocate_inode`](struct.Ext2.html#method.allocate_inode).
    pub fn deallocate_inode(&mut self, inode_number: u32) -> Result<(), Error<Ext2Error>> {
        let mut inode = self.inode(inode_number)?;

        inode.links_count -= if inode.file_type().map_err(FsError::Implementation)? == Type::Directory { 2 } else { 1 };
        if inode.links_count == 0 {
            let file_type = inode.file_type().map_err(FsError::Implementation)?;
            if file_type == Type::Regular
                || file_type == Type::Directory
                || (file_type == Type::SymbolicLink && inode.data_size() >= usize_to_u64(SYMBOLIC_LINK_INODE_STORE_LIMIT))
            {
                let indirected_blocks = inode.indirected_blocks(self)?;
                self.deallocate_blocks(&indirected_blocks.flatten_data_blocks())?;
            }
            self.locate_inode(inode_number, false)?;
        }

        // SAFETY: `inode_address` is the starting address of the inode
        unsafe { Inode::write_on_device(self, inode_number, inode) }
    }
}

/// Main interface to manipulate an ext2 filesystem.
#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Deref, DerefMut)]
pub struct Ext2Fs<Dev: Device<u8, Ext2Error>>(Celled<Ext2<Dev>>);

impl<Dev: Device<u8, Ext2Error>> Clone for Ext2Fs<Dev> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<Dev: Device<u8, Ext2Error>> Ext2Fs<Dev> {
    /// Creates a new [`Ext2Fs`] object from the given device that should contain an ext2 filesystem and a given device ID.
    ///
    /// # Errors
    ///
    /// Returns an [`Error::Device`] if the device cannot be read.
    ///
    /// Returns [`Ext2Error::BadMagic`] if the magic number found in the superblock is not equal to
    /// [`EXT2_SIGNATURE`](superblock::EXT2_SIGNATURE).
    pub fn new(device: Dev, device_id: u32, cache: bool) -> Result<Self, Error<Ext2Error>> {
        Ok(Self(Celled::new(Ext2::new(device, device_id, cache)?)))
    }

    /// Creates a new [`Ext2Fs`] object from the given celled device that should contain an ext2 filesystem and a given device ID.
    ///
    /// # Errors
    ///
    /// Returns an [`Error::Device`] if the device cannot be read.
    ///
    /// Returns [`Ext2Error::BadMagic`] if the magic number found in the superblock is not equal to
    /// [`EXT2_SIGNATURE`](superblock::EXT2_SIGNATURE).
    pub fn new_celled(celled_device: Celled<Dev>, device_id: u32, cache: bool) -> Result<Self, Error<Ext2Error>> {
        Ok(Self(Celled::new(Ext2::new_celled(celled_device, device_id, cache)?)))
    }

    /// Returns a reference to the inner [`Ext2`] object.
    #[must_use]
    pub const fn ext2_interface(&self) -> &Celled<Ext2<Dev>> {
        &self.0
    }

    /// Returns a [`File`](crate::file::File) directly read on this filesystem.
    ///
    /// # Errors
    ///
    /// Returns an [`BadFileType`](Ext2Error::BadFileType) if the type of the file pointed by the given inode is ill-formed.
    ///
    /// Otherwise, returns the same errors as [`Inode::parse`].
    pub fn file(&self, inode_number: u32) -> Result<Ext2TypeWithFile<Dev>, Error<Ext2Error>> {
        let filesystem = self.lock();
        let inode = filesystem.inode(inode_number)?;
        drop(filesystem);
        match inode.file_type().map_err(FsError::Implementation)? {
            Type::Regular => Ok(TypeWithFile::Regular(Regular::new(&self.clone(), inode_number)?)),
            Type::Directory => Ok(TypeWithFile::Directory(Directory::new(&self.clone(), inode_number)?)),
            Type::SymbolicLink => Ok(TypeWithFile::SymbolicLink(SymbolicLink::new(&self.clone(), inode_number)?)),
            Type::Fifo => Ok(TypeWithFile::Fifo(Fifo::new(&self.clone(), inode_number)?)),
            Type::CharacterDevice => Ok(TypeWithFile::CharacterDevice(CharacterDevice::new(&self.clone(), inode_number)?)),
            Type::BlockDevice => Ok(TypeWithFile::BlockDevice(BlockDevice::new(&self.clone(), inode_number)?)),
            Type::Socket => Ok(TypeWithFile::Socket(Socket::new(&self.clone(), inode_number)?)),
        }
    }
}

impl<Dev: Device<u8, Ext2Error>> FileSystem<Directory<Dev>> for Ext2Fs<Dev> {
    fn root(&self) -> Result<Directory<Dev>, Error<<Directory<Dev> as crate::io::Base>::FsError>> {
        self.file(ROOT_DIRECTORY_INODE).and_then(|root| match root {
            TypeWithFile::Directory(root_dir) => Ok(root_dir),
            TypeWithFile::Regular(_)
            | TypeWithFile::SymbolicLink(_)
            | TypeWithFile::Fifo(_)
            | TypeWithFile::CharacterDevice(_)
            | TypeWithFile::BlockDevice(_)
            | TypeWithFile::Socket(_) => Err(Error::Fs(FsError::WrongFileType {
                expected: Type::Directory,
                given: root.into(),
            })),
        })
    }

    fn double_slash_root(&self) -> Result<Directory<Dev>, Error<<Directory<Dev> as crate::io::Base>::FsError>> {
        self.root()
    }
}

#[cfg(test)]
mod test {
    use core::str::FromStr;

    use itertools::Itertools;

    use super::Ext2;
    use super::inode::ROOT_DIRECTORY_INODE;
    use crate::arch::u32_to_usize;
    use crate::file::{Directory, SymbolicLink, Type, TypeWithFile};
    use crate::fs::FileSystem;
    use crate::fs::ext2::Ext2Fs;
    use crate::fs::ext2::block::Block;
    use crate::fs::ext2::block_group::BlockGroupDescriptor;
    use crate::fs::ext2::inode::{Flags, Inode, TypePermissions};
    use crate::io::{Read, Write};
    use crate::path::{Path, UnixStr};
    use crate::permissions::Permissions;
    use crate::tests::{copy_file, new_device_id};
    use crate::types::{Gid, Uid};

    #[test]
    fn base_fs() {
        let device = copy_file("./tests/fs/ext2/base.ext2").unwrap();
        let ext2 = Ext2::new(device, new_device_id(), false).unwrap();
        let root = ext2.inode(ROOT_DIRECTORY_INODE).unwrap();
        assert_eq!(root.file_type().unwrap(), Type::Directory);
    }

    #[test]
    fn fetch_file() {
        let device = copy_file("./tests/fs/ext2/extended.ext2").unwrap();
        let ext2 = Ext2Fs::new(device, new_device_id(), false).unwrap();

        let TypeWithFile::Directory(root) = ext2.file(ROOT_DIRECTORY_INODE).unwrap() else { panic!() };
        let Some(TypeWithFile::Regular(mut big_file)) = root.entry(UnixStr::new("big_file").unwrap()).unwrap() else { panic!() };

        let mut buf = [0_u8; 1024];
        big_file.read(&mut buf).unwrap();
        assert_eq!(buf.into_iter().all_equal_value(), Ok(1));
    }

    #[test]
    fn get_bitmap() {
        let device = copy_file("./tests/fs/ext2/base.ext2").unwrap();
        let ext2 = Ext2::new(device, new_device_id(), false).unwrap();

        assert_eq!(ext2.get_block_bitmap(0).unwrap().length() * 8, u32_to_usize(ext2.superblock().base().blocks_per_group));
    }

    #[test]
    fn free_block_numbers() {
        let device = copy_file("./tests/fs/ext2/base.ext2").unwrap();
        let fs = Ext2Fs::new(device, new_device_id(), false).unwrap();
        let ext2 = fs.ext2_interface().lock();
        let free_blocks = ext2.free_blocks(1_024).unwrap();
        let superblock = ext2.superblock().clone();
        let bitmap = ext2.get_block_bitmap(0).unwrap();

        assert!(free_blocks.iter().all_unique());

        for block in free_blocks {
            assert!(Block::new(fs.clone(), block).is_free(&superblock, &bitmap), "{block}");
        }
    }

    #[test]
    fn free_block_amount() {
        let device = copy_file("./tests/fs/ext2/base.ext2").unwrap();
        let ext2 = Ext2::new(device, 0, false).unwrap();

        for i in 1_u32..1_024 {
            assert_eq!(ext2.free_blocks(i).unwrap().len(), u32_to_usize(i), "{i}");
        }

        let superblock_free_block_count = ext2.superblock().base().free_blocks_count;
        let mut block_group_descriptors_free_block_count = 0;
        for i in 0..ext2.superblock().block_group_count() {
            let bgd = BlockGroupDescriptor::parse(&ext2, i).unwrap();
            block_group_descriptors_free_block_count += u32::from(bgd.free_blocks_count);
        }
        assert_eq!(superblock_free_block_count, block_group_descriptors_free_block_count);
    }

    #[test]
    fn free_block_small_allocation_deallocation() {
        let file = copy_file("./tests/fs/ext2/io_operations.ext2").unwrap();
        let fs = Ext2Fs::new(file, new_device_id(), false).unwrap();
        let mut ext2 = fs.ext2_interface().lock();

        let mut free_blocks = ext2.free_blocks(1_024).unwrap();
        ext2.allocate_blocks(&free_blocks).unwrap();
        free_blocks.push(14849);
        drop(ext2);

        let superblock = fs.lock().superblock().clone();
        for block in &free_blocks {
            let bitmap = fs.lock().get_block_bitmap(superblock.block_group(*block)).unwrap();
            assert!(Block::new(fs.clone(), *block).is_used(&superblock, &bitmap), "Allocation: {block}");
        }

        fs.lock().deallocate_blocks(&free_blocks).unwrap();

        for block in &free_blocks {
            let bitmap = fs.lock().get_block_bitmap(superblock.block_group(*block)).unwrap();
            assert!(Block::new(fs.clone(), *block).is_free(&superblock, &bitmap), "Deallocation: {block}");
        }
    }

    #[test]
    fn free_block_big_allocation_deallocation() {
        let file = copy_file("./tests/fs/ext2/io_operations.ext2").unwrap();
        let fs = Ext2Fs::new(file, new_device_id(), false).unwrap();
        let mut ext2 = fs.ext2_interface().lock();

        let superblock_free_block_count = ext2.superblock().base().free_blocks_count;
        let mut block_group_descriptors_free_block_count = 0;
        for i in 0..ext2.superblock().block_group_count() {
            let bgd = BlockGroupDescriptor::parse(&ext2, i).unwrap();
            block_group_descriptors_free_block_count += u32::from(bgd.free_blocks_count);
        }
        assert_eq!(superblock_free_block_count, block_group_descriptors_free_block_count);

        let free_blocks = ext2.free_blocks(20_000).unwrap();
        ext2.allocate_blocks(&free_blocks).unwrap();
        drop(ext2);

        let superblock = fs.lock().superblock().clone();

        let superblock_free_block_count = superblock.base().free_blocks_count;
        let mut block_group_descriptors_free_block_count = 0;
        for i in 0..superblock.block_group_count() {
            let bgd = BlockGroupDescriptor::parse(&fs.lock(), i).unwrap();
            block_group_descriptors_free_block_count += u32::from(bgd.free_blocks_count);
        }
        assert_eq!(superblock_free_block_count, block_group_descriptors_free_block_count);

        let superblock = fs.lock().superblock().clone();
        for block in &free_blocks {
            let bitmap = fs.lock().get_block_bitmap(superblock.block_group(*block)).unwrap();
            assert!(
                Block::new(fs.clone(), *block).is_used(&superblock, &bitmap),
                "Allocation: {block} ({})",
                superblock.block_group(*block)
            );
        }

        fs.lock().deallocate_blocks(&free_blocks).unwrap();

        for block in &free_blocks {
            let bitmap = fs.lock().get_block_bitmap(superblock.block_group(*block)).unwrap();
            assert!(Block::new(fs.clone(), *block).is_free(&superblock, &bitmap), "Deallocation: {block}");
        }

        let superblock = fs.lock().superblock().clone();
        let superblock_free_block_count = superblock.base().free_blocks_count;
        let mut block_group_descriptors_free_block_count = 0;
        for i in 0..superblock.block_group_count() {
            let bgd = BlockGroupDescriptor::parse(&fs.lock(), i).unwrap();
            block_group_descriptors_free_block_count += u32::from(bgd.free_blocks_count);
        }
        assert_eq!(superblock_free_block_count, block_group_descriptors_free_block_count);
    }

    #[test]
    fn free_inode_allocation_deallocation() {
        let file = copy_file("./tests/fs/ext2/io_operations.ext2").unwrap();
        let fs = Ext2Fs::new(file, new_device_id(), false).unwrap();
        let mut ext2 = fs.ext2_interface().lock();

        let free_inode = ext2.free_inode().unwrap();
        let superblock = ext2.superblock();
        assert!(
            Block::new(fs.clone(), Inode::containing_block(superblock, free_inode))
                .is_used(superblock, &ext2.get_block_bitmap(Inode::block_group(superblock, free_inode)).unwrap())
        );
        let bitmap = ext2.get_inode_bitmap(Inode::block_group(superblock, free_inode)).unwrap();
        assert!(Inode::is_free(free_inode, superblock, &bitmap));

        ext2.allocate_inode(free_inode, TypePermissions::REGULAR_FILE, 0, 0, Flags::empty(), 0, [0; 12])
            .unwrap();
        let superblock = ext2.superblock();
        let bitmap = ext2.get_inode_bitmap(Inode::block_group(superblock, free_inode)).unwrap();
        assert!(Inode::is_used(free_inode, superblock, &bitmap));

        assert_eq!(
            Inode::create(superblock, TypePermissions::REGULAR_FILE, 0, 0, Flags::empty(), 0, [0; 12]),
            Inode::parse(&ext2, free_inode).unwrap()
        );

        ext2.deallocate_inode(free_inode).unwrap();
        let superblock = ext2.superblock();
        let bitmap = ext2.get_inode_bitmap(Inode::block_group(superblock, free_inode)).unwrap();
        assert!(Inode::is_free(free_inode, superblock, &bitmap));
    }

    #[test]
    fn fs_interface() {
        let file = copy_file("./tests/fs/ext2/io_operations.ext2").unwrap();
        let fs = Ext2Fs::new(file, new_device_id(), false).unwrap();

        let root = fs.root().unwrap();

        let Some(TypeWithFile::Regular(mut foo_txt)) = root.entry(UnixStr::new("foo.txt").unwrap()).unwrap() else {
            panic!("foo.txt is a regular file in the root folder");
        };
        assert_eq!(foo_txt.read_all().unwrap(), b"Hello world!\n");

        let Some(TypeWithFile::Directory(mut folder)) = root.entry(UnixStr::new("folder").unwrap()).unwrap() else {
            panic!("folder is a directory in the root folder");
        };
        let Ok(TypeWithFile::Regular(mut ex1_txt)) =
            fs.get_file(&Path::from_str("../folder/ex1.txt").unwrap(), folder.clone(), false)
        else {
            panic!("ex1.txt is a regular file at /folder/ex1.txt");
        };
        ex1_txt.write_all(b"Hello earth!\n").unwrap();

        let TypeWithFile::SymbolicLink(mut boo) = folder
            .add_entry(UnixStr::new("boo.txt").unwrap(), Type::SymbolicLink, Permissions::from_bits_retain(0o777), Uid(0), Gid(0))
            .unwrap()
        else {
            panic!("Could not create a symbolic link");
        };
        boo.set_pointed_file("../baz.txt").unwrap();

        let TypeWithFile::Regular(mut baz_txt) = fs.get_file(&Path::from_str("/folder/boo.txt").unwrap(), root, true).unwrap()
        else {
            panic!("Could not retrieve baz.txt from boo.txt");
        };
        assert_eq!(ex1_txt.read_all().unwrap(), baz_txt.read_all().unwrap());
    }
}
