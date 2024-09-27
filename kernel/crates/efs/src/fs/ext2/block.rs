//! Interface to manipulate blocks.
//!
//! A block is a contiguous part of the disk space. For a given filesystem, all the blocks have the same size, indicated in the
//! [`Superblock`].
//!
//! See [the OSDev wiki](https://wiki.osdev.org/Ext2#What_is_a_Block.3F) for more information.

use alloc::vec;
use alloc::vec::Vec;

use super::Ext2Fs;
use super::error::Ext2Error;
use super::superblock::Superblock;
use crate::arch::u32_to_usize;
use crate::dev::Device;
use crate::dev::sector::Address;
use crate::error::Error;
use crate::fs::error::FsError;
use crate::io::{Base, Read, Seek, SeekFrom, Write};

/// An ext2 block.
///
/// The [`Device`] is splitted in contiguous ext2 blocks that have all the same size in bytes. This is **NOT** the block as in block
/// device, here "block" always refers to ext2's blocks. They start at 0, so the `n`th block will start at the adress `n *
/// block_size`. Thus, a block is entirely described by its number.
#[derive(Clone)]
pub struct Block<Dev: Device<u8, Ext2Error>> {
    /// Block number.
    number: u32,

    /// Ext2 object associated with the device containing this block.
    filesystem: Ext2Fs<Dev>,

    /// Offset for the I/O operations.
    io_offset: u32,
}

impl<Dev: Device<u8, Ext2Error>> Block<Dev> {
    /// Returns a [`Block`] from its number and an [`Ext2Fs`] instance.
    #[must_use]
    pub const fn new(filesystem: Ext2Fs<Dev>, number: u32) -> Self {
        Self {
            number,
            filesystem,
            io_offset: 0,
        }
    }

    /// Returns the containing block group of this block.
    #[must_use]
    pub const fn block_group(&self, superblock: &Superblock) -> u32 {
        superblock.block_group(self.number)
    }

    /// Returns the offset of this block in its containing block group.
    #[must_use]
    pub const fn group_index(&self, superblock: &Superblock) -> u32 {
        superblock.group_index(self.number)
    }

    /// Reads all the content from this block and returns it in a vector.
    ///
    /// The offset for the I/O operations is reset at 0 at the end of this function.
    ///
    /// # Errors
    ///
    /// Returns an [`Error::Device`] if the device cannot be read.
    pub fn read_all(&mut self) -> Result<Vec<u8>, Error<Ext2Error>> {
        let block_size = self.filesystem.lock().superblock().block_size();
        let mut buffer = vec![0_u8; u32_to_usize(block_size)];
        self.seek(SeekFrom::Start(0))?;
        self.read(&mut buffer)?;
        self.seek(SeekFrom::Start(0))?;
        Ok(buffer)
    }

    /// Returns whether this block is currently free or not from the block bitmap in which the block resides.
    ///
    /// The `bitmap` argument is usually the result of the method [`get_block_bitmap`](../struct.Ext2.html#method.get_block_bitmap).
    #[allow(clippy::indexing_slicing)]
    #[must_use]
    pub const fn is_free(&self, superblock: &Superblock, bitmap: &[u8]) -> bool {
        let index = self.group_index(superblock) / 8;
        let offset = (self.number - superblock.base().first_data_block) % 8;
        bitmap[index as usize] >> offset & 1 == 0
    }

    /// Returns whether this block is currently used or not from the block bitmap in which the block resides.
    ///
    /// The `bitmap` argument is usually the result of the method [`get_block_bitmap`](../struct.Ext2.html#method.get_block_bitmap).
    #[allow(clippy::indexing_slicing)]
    #[must_use]
    pub const fn is_used(&self, superblock: &Superblock, bitmap: &[u8]) -> bool {
        !self.is_free(superblock, bitmap)
    }

    /// Sets the current block usage in the block bitmap, and updates the superblock accordingly.
    ///
    /// # Errors
    ///
    /// Returns an [`BlockAlreadyInUse`](Ext2Error::BlockAlreadyInUse) error if the given block was already in use.
    ///
    /// Returns an [`BlockAlreadyFree`](Ext2Error::BlockAlreadyFree) error if the given block was already free.
    ///
    /// Returns an [`Error::Device`] if the device cannot be written.
    fn set_usage(&self, usage: bool) -> Result<(), Error<Ext2Error>> {
        self.filesystem.lock().locate_blocks(&[self.number], usage)
    }

    /// Sets the current block as free in the block bitmap, and updates the superblock accordingly.
    ///
    /// # Errors
    ///
    /// Returns an [`BlockAlreadyFree`](Ext2Error::BlockAlreadyFree) error if the given block was already free.
    ///
    /// Returns an [`Error::Device`] if the device cannot be written.
    pub fn set_free(&mut self) -> Result<(), Error<Ext2Error>> {
        self.set_usage(false)
    }

    /// Sets the current block as used in the block bitmap, and updates the superblock accordingly.
    ///
    /// # Errors
    ///
    /// Returns an [`BlockAlreadyInUse`](Ext2Error::BlockAlreadyInUse) error if the given block was already in use.
    ///
    /// Returns an [`Error::Device`] if the device cannot be written.
    pub fn set_used(&mut self) -> Result<(), Error<Ext2Error>> {
        self.set_usage(true)
    }
}

impl<Dev: Device<u8, Ext2Error>> From<Block<Dev>> for u32 {
    fn from(block: Block<Dev>) -> Self {
        block.number
    }
}

impl<Dev: Device<u8, Ext2Error>> Base for Block<Dev> {
    type FsError = Ext2Error;
}

impl<Dev: Device<u8, Ext2Error>> Read for Block<Dev> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error<Self::FsError>> {
        let fs = self.filesystem.lock();
        let mut device = fs.device.lock();

        let length = u32_to_usize(fs.superblock().block_size() - self.io_offset).min(buf.len());
        let starting_addr =
            Address::new(u32_to_usize(self.number) * u32_to_usize(fs.superblock().block_size()) + u32_to_usize(self.io_offset));
        let slice = device.slice(starting_addr..starting_addr + length)?;
        buf.clone_from_slice(slice.as_ref());

        // SAFETY: `length <= block_size << u32::MAX`
        self.io_offset += unsafe { u32::try_from(length).unwrap_unchecked() };

        Ok(length)
    }
}

impl<Dev: Device<u8, Ext2Error>> Write for Block<Dev> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error<Self::FsError>> {
        let fs = self.filesystem.lock();
        let mut device = fs.device.lock();

        let length = u32_to_usize(fs.superblock().block_size() - self.io_offset).min(buf.len());
        let starting_addr =
            Address::new(u32_to_usize(self.number) * u32_to_usize(fs.superblock().block_size()) + u32_to_usize(self.io_offset));
        let mut slice = device.slice(starting_addr..starting_addr + length)?;
        // SAFETY: buf size is at least length
        slice.clone_from_slice(unsafe { buf.get_unchecked(..length) });
        let commit = slice.commit();
        device.commit(commit)?;

        // SAFETY: `length <= block_size < u32::MAX`
        self.io_offset += unsafe { u32::try_from(length).unwrap_unchecked() };

        Ok(length)
    }

    fn flush(&mut self) -> Result<(), Error<Self::FsError>> {
        Ok(())
    }
}

impl<Dev: Device<u8, Ext2Error>> Seek for Block<Dev> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Error<Self::FsError>> {
        let fs = self.filesystem.lock();

        let block_size = i64::from(fs.superblock().block_size());
        let previous_offset = self.io_offset;
        match pos {
            SeekFrom::Start(offset) => {
                self.io_offset =
                    u32::try_from(offset).map_err(|_err| FsError::Implementation(Ext2Error::OutOfBounds(offset.into())))?;
            },
            SeekFrom::End(back_offset) => {
                self.io_offset = TryInto::<u32>::try_into(block_size - back_offset)
                    .map_err(|_err| FsError::Implementation(Ext2Error::OutOfBounds(i128::from(block_size - back_offset))))?;
            },
            SeekFrom::Current(add_offset) => {
                self.io_offset = (i64::from(previous_offset) + add_offset).try_into().map_err(|_err| {
                    FsError::Implementation(Ext2Error::OutOfBounds(i128::from(i64::from(previous_offset) + add_offset)))
                })?;
            },
        };

        if self.io_offset >= fs.superblock().block_size() {
            Err(Error::Fs(FsError::Implementation(Ext2Error::OutOfBounds(self.io_offset.into()))))
        } else {
            Ok(previous_offset.into())
        }
    }
}

#[cfg(test)]
mod test {
    use alloc::vec;
    use std::fs::File;

    use crate::celled::Celled;
    use crate::dev::Device;
    use crate::dev::sector::Address;
    use crate::fs::ext2::Ext2Fs;
    use crate::fs::ext2::block::Block;
    use crate::fs::ext2::block_group::BlockGroupDescriptor;
    use crate::fs::ext2::error::Ext2Error;
    use crate::fs::ext2::superblock::Superblock;
    use crate::io::{Read, Seek, SeekFrom, Write};
    use crate::tests::{copy_file, new_device_id};

    #[test]
    fn block_read() {
        const BLOCK_NUMBER: u32 = 2;

        let file = copy_file("./tests/fs/ext2/io_operations.ext2").unwrap();
        let celled_file = Celled::new(file);
        let superblock = Superblock::parse(&celled_file).unwrap();

        let block_starting_addr = Address::new((BLOCK_NUMBER * superblock.block_size()).try_into().unwrap());
        let slice = <File as Device<u8, Ext2Error>>::slice(
            &mut celled_file.lock(),
            block_starting_addr + 123..block_starting_addr + 123 + 59,
        )
        .unwrap()
        .commit();

        let ext2 = Ext2Fs::new_celled(celled_file, 0, false).unwrap();
        let mut block = Block::new(ext2, BLOCK_NUMBER);
        block.seek(SeekFrom::Start(123)).unwrap();
        let mut buffer_auto = [0_u8; 59];
        block.read(&mut buffer_auto).unwrap();

        assert_eq!(buffer_auto, slice.as_ref());
    }

    #[test]
    fn block_write() {
        const BLOCK_NUMBER: u32 = 10_234;

        let file = copy_file("./tests/fs/ext2/io_operations.ext2").unwrap();
        let ext2 = Ext2Fs::new(file, new_device_id(), false).unwrap();
        let superblock = ext2.lock().superblock().clone();

        let mut block = Block::new(ext2, BLOCK_NUMBER);
        let mut buffer = vec![0_u8; usize::try_from(superblock.block_size()).unwrap() - 123];
        buffer[..59].copy_from_slice(&[1_u8; 59]);
        block.seek(SeekFrom::Start(123)).unwrap();
        block.write(&buffer).unwrap();

        let mut start = vec![0_u8; 123];
        start.append(&mut buffer);
        assert_eq!(block.read_all().unwrap(), start);
    }

    #[test]
    fn block_set_free() {
        // This block should not be free
        const BLOCK_NUMBER: u32 = 9;

        let file = copy_file("./tests/fs/ext2/io_operations.ext2").unwrap();
        let ext2 = Ext2Fs::new(file, new_device_id(), false).unwrap();
        let superblock = ext2.lock().superblock().clone();

        let mut block = Block::new(ext2.clone(), BLOCK_NUMBER);
        let block_group = block.block_group(&superblock);

        let fs = ext2.lock();
        let block_group_descriptor = BlockGroupDescriptor::parse(&fs, block_group).unwrap();
        let free_block_count = block_group_descriptor.free_blocks_count;

        let bitmap = fs.get_block_bitmap(block_group).unwrap();

        drop(fs);

        assert!(block.is_used(&superblock, &bitmap));

        block.set_free().unwrap();

        let fs = ext2.lock();
        let new_free_block_count = BlockGroupDescriptor::parse(&fs, block.block_group(&superblock))
            .unwrap()
            .free_blocks_count;

        assert!(block.is_free(&superblock, &fs.get_block_bitmap(block_group).unwrap()));
        assert_eq!(free_block_count + 1, new_free_block_count);
    }

    #[test]
    fn block_set_used() {
        // This block should not be used
        const BLOCK_NUMBER: u32 = 1920;

        let file = copy_file("./tests/fs/ext2/io_operations.ext2").unwrap();
        let ext2 = Ext2Fs::new(file, new_device_id(), false).unwrap();
        let superblock = ext2.lock().superblock().clone();

        let mut block = Block::new(ext2.clone(), BLOCK_NUMBER);
        let block_group = block.block_group(&superblock);

        let fs = ext2.lock();

        let block_group_descriptor = BlockGroupDescriptor::parse(&fs, block_group).unwrap();
        let free_block_count = block_group_descriptor.free_blocks_count;

        let bitmap = fs.get_block_bitmap(block_group).unwrap();

        assert!(block.is_free(&superblock, &bitmap));

        drop(fs);

        block.set_used().unwrap();

        let fs = ext2.lock();
        let new_free_block_count = BlockGroupDescriptor::parse(&fs, block.block_group(&superblock))
            .unwrap()
            .free_blocks_count;

        assert!(block.is_used(&superblock, &fs.get_block_bitmap(block_group).unwrap()));
        assert_eq!(free_block_count - 1, new_free_block_count);
    }
}
