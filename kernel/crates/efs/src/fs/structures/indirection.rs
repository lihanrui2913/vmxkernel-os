//! Manipulation of indirection in extended filesystems (ext2, ext3 and ext4), for the major part for inodes' blocks.
//!
//! See [this Wikipedia section](https://en.wikipedia.org/wiki/Ext2#Inodes) for more information.

use alloc::vec::Vec;

use itertools::Itertools;

use crate::arch::u32_to_usize;

/// Block indirections.
///
/// See [*The Second Extended Filesystem* book](https://www.nongnu.org/ext2-doc/ext2.html#i-block) for more information.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Indirection {
    /// Directly accessible blocks.
    Direct,

    /// Simply indirected blocks.
    Simple,

    /// Doubly indirected blocks.
    Double,

    /// Triply indirected blocks.
    Triple,
}

/// Type alias representing direct blocks.
pub type DirectBlocks = Vec<u32>;

/// Type alias representing a single indirection block.
#[allow(clippy::module_name_repetitions)]
pub type SimpleIndirection = (u32, Vec<u32>);

/// Type alias representing a double indirection block.
#[allow(clippy::module_name_repetitions)]
pub type DoubleIndirection = (u32, Vec<SimpleIndirection>);

/// Type alias representing a triple indirection block.
#[allow(clippy::module_name_repetitions)]
pub type TripleIndirection = (u32, Vec<DoubleIndirection>);

/// Represents a structure that contains indirections.
trait Indirected {
    /// Fetches the block at given `offset` knowing the number of blocks in each group.
    fn resolve_indirection(&self, offset: u32, blocks_per_indirection: u32) -> Option<u32>;
}

impl Indirected for DirectBlocks {
    fn resolve_indirection(&self, offset: u32, _blocks_per_indirection: u32) -> Option<u32> {
        self.get(u32_to_usize(offset)).copied()
    }
}

impl Indirected for SimpleIndirection {
    fn resolve_indirection(&self, offset: u32, _blocks_per_indirection: u32) -> Option<u32> {
        self.1.get(u32_to_usize(offset)).copied()
    }
}

impl Indirected for DoubleIndirection {
    fn resolve_indirection(&self, offset: u32, blocks_per_indirection: u32) -> Option<u32> {
        let double_indirection_index = offset / blocks_per_indirection;
        let simple_indirection_index = offset % blocks_per_indirection;
        self.1.get(u32_to_usize(double_indirection_index)).and_then(|simple_indirection_block| {
            simple_indirection_block
                .1
                .resolve_indirection(simple_indirection_index, blocks_per_indirection)
        })
    }
}

impl Indirected for TripleIndirection {
    fn resolve_indirection(&self, offset: u32, blocks_per_indirection: u32) -> Option<u32> {
        let triple_indirection_index = offset / (blocks_per_indirection * blocks_per_indirection);
        let double_indirection_index = offset % (blocks_per_indirection * blocks_per_indirection);
        self.1.get(u32_to_usize(triple_indirection_index)).and_then(|double_indirection_block| {
            double_indirection_block.resolve_indirection(double_indirection_index, blocks_per_indirection)
        })
    }
}

/// Type for data blocks in an inode.
///
/// Only contains the real data blocks (with a number different than 0).
///
/// The parameter `DBPC` is the maximal number of direct block pointers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndirectedBlocks<const DBPC: u32> {
    /// Number of blocks contained in each indirection.
    ///
    /// In ext2 filesystems, this always should be equal to `superblock.block_size() / 4`.
    pub(crate) blocks_per_indirection: u32,

    /// The direct block numbers.
    pub(crate) direct_blocks: DirectBlocks,

    /// The singly indirected block numbers.
    pub(crate) singly_indirected_blocks: SimpleIndirection,

    /// The doubly indirected block numbers.
    pub(crate) doubly_indirected_blocks: DoubleIndirection,

    /// The triply indirected block numbers.
    pub(crate) triply_indirected_blocks: TripleIndirection,
}

impl<const DBPC: u32> IndirectedBlocks<DBPC> {
    /// Creates a new instance from complete list of data blocks.
    #[must_use]
    pub(crate) const fn new(
        blocks_per_indirection: u32,
        direct_blocks: DirectBlocks,
        singly_indirected_blocks: SimpleIndirection,
        doubly_indirected_blocks: DoubleIndirection,
        triply_indirected_blocks: TripleIndirection,
    ) -> Self {
        Self {
            blocks_per_indirection,
            direct_blocks,
            singly_indirected_blocks,
            doubly_indirected_blocks,
            triply_indirected_blocks,
        }
    }

    /// Returns every data block with the indirected blocks.
    #[must_use]
    pub fn blocks(self) -> (DirectBlocks, SimpleIndirection, DoubleIndirection, TripleIndirection) {
        (self.direct_blocks, self.singly_indirected_blocks, self.doubly_indirected_blocks, self.triply_indirected_blocks)
    }

    /// Returns the complete list of block numbers containing this inode's data (indirect blocks are not considered) in a single
    /// continuous vector.
    #[must_use]
    pub fn flatten_data_blocks_with_indirection(&self) -> Vec<(u32, (Indirection, u32))> {
        let block_with_indirection = |indirection| {
            // SAFETY: the total number of blocks is stored on a u32
            move |(index, block): (usize, &u32)| (*block, (indirection, unsafe { u32::try_from(index).unwrap_unchecked() }))
        };

        let mut blocks = self
            .direct_blocks
            .iter()
            .enumerate()
            .map(block_with_indirection(Indirection::Direct))
            .collect_vec();

        blocks.append(
            &mut self
                .singly_indirected_blocks
                .1
                .iter()
                .enumerate()
                .map(block_with_indirection(Indirection::Simple))
                .collect_vec(),
        );

        blocks.append(
            &mut self
                .doubly_indirected_blocks
                .1
                .iter()
                .enumerate()
                .flat_map(|(simple_indirection_index, (_, blocks))| {
                    blocks
                        .iter()
                        .enumerate()
                        .map(|(index, block)| {
                            (
                                *block,
                                (
                                    Indirection::Double,
                                    // SAFETY: the total number of blocks is stored on a u32
                                    unsafe { u32::try_from(simple_indirection_index).unwrap_unchecked() }
                                        * self.blocks_per_indirection
                                        // SAFETY: the total number of blocks is stored on a u32
                                        + unsafe { u32::try_from(index).unwrap_unchecked() },
                                ),
                            )
                        })
                        .collect_vec()
                })
                .collect_vec(),
        );

        blocks.append(
            &mut self
                .triply_indirected_blocks
                .1
                .iter()
                .enumerate()
                .flat_map(|(double_indirection_index, (_, indirected_blocks))| {
                    indirected_blocks
                        .iter()
                        .enumerate()
                        .flat_map(|(simple_indirection_index, (_, blocks))| {
                            blocks
                                .iter()
                                .enumerate()
                                .map(|(index, block)| {
                                    (
                                        *block,
                                        (
                                            Indirection::Triple,
                                            // SAFETY: the total number of blocks is stored on a u32
                                            unsafe { u32::try_from(double_indirection_index).unwrap_unchecked() }
                                                * self.blocks_per_indirection
                                                * self.blocks_per_indirection
                                                // SAFETY: the total number of blocks is stored on a u32
                                                + unsafe { u32::try_from(simple_indirection_index).unwrap_unchecked() }
                                                    * self.blocks_per_indirection
                                                    // SAFETY: the total number of blocks is stored on a u32
                                                    + unsafe { u32::try_from(index).unwrap_unchecked() },
                                        ),
                                    )
                                })
                                .collect_vec()
                        })
                        .collect_vec()
                })
                .collect_vec(),
        );

        blocks
    }

    /// Returns the complete list of block numbers containing this inode's data (indirect blocks are not considered) in a single
    /// continuous vector.
    #[must_use]
    pub fn flatten_data_blocks(&self) -> Vec<u32> {
        self.flatten_data_blocks_with_indirection()
            .into_iter()
            .map(|(block, _)| block)
            .collect_vec()
    }

    /// Returns the indirection and the remaining offset in this indirection to fetch the block at the given `offset`.
    ///
    /// Returns [`None`] if the offset points a block outside the range of this structure.
    #[allow(clippy::suspicious_operation_groupings)]
    #[must_use]
    pub const fn block_at_offset_remainging_in_indirection(offset: u32, blocks_per_indirection: u32) -> Option<(Indirection, u32)> {
        if offset < 12 {
            Some((Indirection::Direct, offset))
        } else if offset < DBPC + blocks_per_indirection {
            Some((Indirection::Simple, offset - DBPC))
        } else if offset < DBPC + blocks_per_indirection + blocks_per_indirection * blocks_per_indirection {
            Some((Indirection::Double, offset - (DBPC + blocks_per_indirection)))
        } else if offset
            < DBPC
                + blocks_per_indirection
                + blocks_per_indirection * blocks_per_indirection
                + blocks_per_indirection * blocks_per_indirection * blocks_per_indirection
        {
            Some((Indirection::Triple, offset - (DBPC + blocks_per_indirection + blocks_per_indirection * blocks_per_indirection)))
        } else {
            None
        }
    }

    /// Returns the block at the given offset in the given indirection.
    ///
    /// This is easily usable in pair with
    /// [`block_at_offset_remainging_in_indirection`](struct.IndirectedBlocks.html#method.block_at_offset_remainging_in_indirection)
    /// or with [`last_data_block_allocated`](struct.IndirectedBlocks.html#method.last_data_block_allocated).
    #[must_use]
    pub fn block_at_offset_in_indirection(&self, indirection: Indirection, offset: u32) -> Option<u32> {
        match indirection {
            Indirection::Direct => self.direct_blocks.resolve_indirection(offset, self.blocks_per_indirection),
            Indirection::Simple => self.singly_indirected_blocks.resolve_indirection(offset, self.blocks_per_indirection),
            Indirection::Double => self.doubly_indirected_blocks.resolve_indirection(offset, self.blocks_per_indirection),
            Indirection::Triple => self.triply_indirected_blocks.resolve_indirection(offset, self.blocks_per_indirection),
        }
    }

    /// Returns the block at the given offset.
    #[must_use]
    pub fn block_at_offset(&self, offset: u32) -> Option<u32> {
        let (indirection, remaining_offset) = Self::block_at_offset_remainging_in_indirection(offset, self.blocks_per_indirection)?;
        self.block_at_offset_in_indirection(indirection, remaining_offset)
    }

    /// Returns the last allocated block of the complete structure, if it exists, with its indirection and its the remaining offset
    /// in the redirection.
    #[must_use]
    pub fn last_data_block_allocated(&self) -> Option<(u32, (Indirection, u32))> {
        let last_triply_indirected = self
            .triply_indirected_blocks
            .1
            .last()
            .and_then(|(_, doubly_indirected_blocks)| {
                doubly_indirected_blocks.last().map(|(_, singly_indirected_blocks)| {
                    singly_indirected_blocks.last().map(|block| {
                        (
                            // SAFETY: `double_indirection_index < blocks_per_indirection << u32::MAX`
                            unsafe { u32::try_from(self.triply_indirected_blocks.1.len() - 1).unwrap_unchecked() }
                                    * self.blocks_per_indirection
                                    * self.blocks_per_indirection
                                    // SAFETY: `single_indirection_index < blocks_per_indirection << u32::MAX`
                                    + unsafe { u32::try_from(doubly_indirected_blocks.len() - 1).unwrap_unchecked() }
                                        * self.blocks_per_indirection
                                    // SAFETY: `direct_block_index < blocks_per_indirection << u32::MAX`
                                    + unsafe { u32::try_from(singly_indirected_blocks.len() - 1).unwrap_unchecked() },
                            *block,
                        )
                    })
                })
            })
            .flatten();

        if let Some((offset, block)) = last_triply_indirected {
            return Some((block, (Indirection::Triple, offset)));
        }

        let last_doubly_indirected = self.doubly_indirected_blocks.1.last().and_then(|(_, singly_indirected_blocks)| {
            singly_indirected_blocks.last().map(|block| {
                (
                    // SAFETY: `single_indirection_index < blocks_per_indirection << u32::MAX`
                    unsafe { u32::try_from(self.doubly_indirected_blocks.1.len() - 1).unwrap_unchecked() } * self.blocks_per_indirection
                            // SAFETY: `direct_block_index < blocks_per_indirection << u32::MAX`
                            + unsafe { u32::try_from(singly_indirected_blocks.len() - 1).unwrap_unchecked() },
                    *block,
                )
            })
        });

        if let Some((offset, block)) = last_doubly_indirected {
            return Some((block, (Indirection::Double, offset)));
        }

        let last_singly_indirected = self
            .singly_indirected_blocks
            .1
            .last()
            // SAFETY: `direct_block_index < blocks_per_indirection << u32::MAX`
            .map(|block| (unsafe { u32::try_from(self.singly_indirected_blocks.1.len() - 1).unwrap_unchecked() }, *block));

        if let Some((offset, block)) = last_singly_indirected {
            return Some((block, (Indirection::Simple, offset)));
        }

        self.direct_blocks
            .iter()
            .enumerate()
            .last()
            // SAFETY: `direct_block_index < 12 << u32::MAX`
            .map(|(index, block)| (*block, (Indirection::Direct, unsafe { u32::try_from(index).unwrap_unchecked() })))
    }

    /// Returns the total number of data blocks.
    #[must_use]
    pub fn data_block_count(&self) -> u32 {
        match self.last_data_block_allocated() {
            None => 0,
            Some((_, (indirection, index))) => {
                1 + index
                    + match indirection {
                        Indirection::Direct => 0,
                        Indirection::Simple => DBPC,
                        Indirection::Double => DBPC + self.blocks_per_indirection,
                        Indirection::Triple => {
                            DBPC + self.blocks_per_indirection + self.blocks_per_indirection * self.blocks_per_indirection
                        },
                    }
            },
        }
    }

    /// Returns the number of necessary indirection blocks to have `data_block_count` blocks of data.
    ///
    /// Returns [`None`] if the given number of data blocks cannot fit on this structure.
    #[must_use]
    pub const fn necessary_indirection_block_count(mut data_block_count: u32, blocks_per_indirection: u32) -> u32 {
        if data_block_count <= DBPC {
            return 0;
        }

        data_block_count -= DBPC;

        if data_block_count <= blocks_per_indirection {
            return 1;
        }

        data_block_count -= blocks_per_indirection;

        if data_block_count <= blocks_per_indirection * blocks_per_indirection {
            return 1 + 1 + 1 + (data_block_count - 1) / blocks_per_indirection;
        }

        data_block_count -= blocks_per_indirection * blocks_per_indirection;

        1 + 1
            + blocks_per_indirection
            + 1
            + 1
            + (data_block_count - 1) / blocks_per_indirection
            + 1
            + ((data_block_count - 1) / blocks_per_indirection) / blocks_per_indirection
    }

    /// Returns the total number of indirection blocks.
    #[must_use]
    pub fn indirection_block_count(&self) -> u32 {
        Self::necessary_indirection_block_count(self.data_block_count(), self.blocks_per_indirection)
    }

    /// Appends the given `blocks` to the indirection blocks.
    ///
    /// The given blocks are used **both for data and indirection blocks**.
    ///
    /// If more blocks than necessary to complete all the structure, only the first one will be used.
    pub fn append_blocks(&mut self, blocks: &[u32]) {
        let blocks_per_indirection = u32_to_usize(self.blocks_per_indirection);

        let blocks_iterator = &mut blocks.iter();

        if self.direct_blocks.len() < u32_to_usize(DBPC) {
            self.direct_blocks
                .append(&mut blocks_iterator.take(u32_to_usize(DBPC) - self.direct_blocks.len()).copied().collect_vec());
        }

        if self.singly_indirected_blocks.0 == 0 {
            self.singly_indirected_blocks.0 = blocks_iterator.next().copied().unwrap_or_default();
        }

        if blocks_iterator.is_empty() {
            return;
        }

        if self.singly_indirected_blocks.1.len() < blocks_per_indirection {
            self.singly_indirected_blocks.1.append(
                &mut blocks_iterator
                    .take(blocks_per_indirection - self.singly_indirected_blocks.1.len())
                    .copied()
                    .collect_vec(),
            );
        }

        if self.doubly_indirected_blocks.0 == 0 {
            self.doubly_indirected_blocks.0 = blocks_iterator.next().copied().unwrap_or_default();
        }

        if blocks_iterator.is_empty() {
            return;
        }

        if self.doubly_indirected_blocks.1.len() <= blocks_per_indirection {
            match self.doubly_indirected_blocks.1.last_mut() {
                None => {},
                Some((_, data_blocks)) => {
                    if data_blocks.len() < blocks_per_indirection {
                        data_blocks
                            .append(&mut blocks_iterator.take(blocks_per_indirection - data_blocks.len()).copied().collect_vec());
                    }
                },
            }

            while self.doubly_indirected_blocks.1.len() < blocks_per_indirection
                && let Some(block) = blocks_iterator.next()
            {
                let indirection_block = (*block, blocks_iterator.take(blocks_per_indirection).copied().collect_vec());
                self.doubly_indirected_blocks.1.push(indirection_block);
            }
        }

        if self.triply_indirected_blocks.0 == 0 {
            self.triply_indirected_blocks.0 = blocks_iterator.next().copied().unwrap_or_default();
        }

        if blocks_iterator.is_empty() {
            return;
        }

        if self.triply_indirected_blocks.1.len() <= blocks_per_indirection {
            match self.triply_indirected_blocks.1.last_mut() {
                None => {},
                Some((_, indirected_blocks)) => {
                    if indirected_blocks.len() < blocks_per_indirection {
                        match indirected_blocks.last_mut() {
                            None => {},
                            Some((_, data_blocks)) => {
                                if data_blocks.len() < blocks_per_indirection {
                                    data_blocks.append(
                                        &mut blocks_iterator
                                            .take(blocks_per_indirection - data_blocks.len())
                                            .copied()
                                            .collect_vec(),
                                    );
                                }
                            },
                        }

                        while indirected_blocks.len() < blocks_per_indirection
                            && let Some(block) = blocks_iterator.next()
                        {
                            let indirection_block = (*block, blocks_iterator.take(blocks_per_indirection).copied().collect_vec());
                            indirected_blocks.push(indirection_block);
                        }
                    }
                },
            }

            while self.triply_indirected_blocks.1.len() < blocks_per_indirection
                && let Some(block) = blocks_iterator.next()
            {
                let mut doubly_indirection_block = (*block, Vec::new());
                while let Some(block) = blocks_iterator.next()
                    && doubly_indirection_block.1.len() < blocks_per_indirection
                {
                    let indirection_block = (*block, blocks_iterator.take(blocks_per_indirection).copied().collect_vec());
                    doubly_indirection_block.1.push(indirection_block);
                }
                self.triply_indirected_blocks.1.push(doubly_indirection_block);
            }
        }
    }

    /// Truncates the end of the indirected blocks at the `n`th data block (excluded).
    ///
    /// In other words, only the `n` first data blocks will be kept.
    pub fn truncate_back_data_blocks(&mut self, mut n: u32) {
        if n > self.data_block_count() {
            return;
        }

        if n <= DBPC {
            self.direct_blocks.drain(u32_to_usize(n)..);
            self.singly_indirected_blocks = (0, Vec::new());
            self.doubly_indirected_blocks = (0, Vec::new());
            self.triply_indirected_blocks = (0, Vec::new());
            return;
        }

        n -= DBPC;

        if n <= self.blocks_per_indirection {
            self.singly_indirected_blocks.1.drain(u32_to_usize(n)..);
            self.doubly_indirected_blocks = (0, Vec::new());
            self.triply_indirected_blocks = (0, Vec::new());
            return;
        }

        n -= self.blocks_per_indirection;

        if n <= self.blocks_per_indirection * self.blocks_per_indirection {
            if let Some((_, blocks)) = self.doubly_indirected_blocks.1.get_mut(u32_to_usize(n / self.blocks_per_indirection)) {
                blocks.drain(u32_to_usize(n % self.blocks_per_indirection)..);
            }
            self.doubly_indirected_blocks
                .1
                .drain(u32_to_usize((n - 1) / self.blocks_per_indirection) + 1..);
            self.triply_indirected_blocks = (0, Vec::new());
            return;
        }

        n -= self.blocks_per_indirection * self.blocks_per_indirection;

        if let Some((_, indirected_blocks)) = self
            .triply_indirected_blocks
            .1
            .get_mut(u32_to_usize((n / self.blocks_per_indirection) / self.blocks_per_indirection))
        {
            if let Some((_, blocks)) = indirected_blocks.get_mut(u32_to_usize(n / self.blocks_per_indirection)) {
                blocks.drain(u32_to_usize(n % self.blocks_per_indirection)..);
            }
            indirected_blocks.drain(u32_to_usize((n - 1) / self.blocks_per_indirection) + 1..);
        }
        self.triply_indirected_blocks
            .1
            .drain(u32_to_usize(((n - 1) / self.blocks_per_indirection) / self.blocks_per_indirection) + 1..);
    }

    /// Truncates the start of the indirected blocks at the `n`th data block (excluded).
    ///
    /// In other words, all the data blocks but the `n` firsts will be kept.
    #[must_use]
    pub fn truncate_front_data_blocks(self, mut n: u32) -> SymmetricDifference<DBPC> {
        let blocks_per_indirection = self.blocks_per_indirection;

        let mut symmetric_difference = SymmetricDifference {
            blocks_per_indirection,
            direct_blocks: (0, self.direct_blocks),
            singly_indirected_blocks: (0, self.singly_indirected_blocks),
            doubly_indirected_blocks: (0, self.doubly_indirected_blocks),
            triply_indirected_blocks: (0, self.triply_indirected_blocks),
        };

        if n < DBPC {
            symmetric_difference.direct_blocks.0 = u32_to_usize(n);
            symmetric_difference.direct_blocks.1.drain(..u32_to_usize(n));
            return symmetric_difference;
        }

        symmetric_difference.direct_blocks = (0, Vec::new());
        n -= DBPC;

        if n < blocks_per_indirection {
            symmetric_difference.singly_indirected_blocks.0 = u32_to_usize(n);
            symmetric_difference.singly_indirected_blocks.1.1.drain(..u32_to_usize(n));
            return symmetric_difference;
        }

        symmetric_difference.singly_indirected_blocks = (0, (0, Vec::new()));
        n -= blocks_per_indirection;

        if n < blocks_per_indirection * blocks_per_indirection {
            symmetric_difference.doubly_indirected_blocks.0 = u32_to_usize(n);
            symmetric_difference
                .doubly_indirected_blocks
                .1
                .1
                .drain(..u32_to_usize(n / blocks_per_indirection));
            if let Some((_, blocks)) = symmetric_difference.doubly_indirected_blocks.1.1.first_mut() {
                blocks.drain(..u32_to_usize(n % blocks_per_indirection));
            }

            return symmetric_difference;
        }

        symmetric_difference.doubly_indirected_blocks = (0, (0, Vec::new()));
        n -= blocks_per_indirection * blocks_per_indirection;

        if n < blocks_per_indirection * blocks_per_indirection * blocks_per_indirection {
            symmetric_difference.triply_indirected_blocks.0 = u32_to_usize(n);
            symmetric_difference
                .triply_indirected_blocks
                .1
                .1
                .drain(..u32_to_usize((n / blocks_per_indirection) / blocks_per_indirection));
            if let Some((_, indirected_blocks)) = symmetric_difference.triply_indirected_blocks.1.1.first_mut() {
                indirected_blocks
                    .drain(..u32_to_usize((n % (blocks_per_indirection * blocks_per_indirection)) / blocks_per_indirection));
                if let Some((_, blocks)) = indirected_blocks.first_mut() {
                    blocks.drain(..u32_to_usize(n % blocks_per_indirection));
                }
            }
            return symmetric_difference;
        }

        symmetric_difference.triply_indirected_blocks = (0, (0, Vec::new()));

        symmetric_difference
    }

    /// Returns the result of `self.append_blocks(blocks)` and the [`SymmetricDifference`] between `self` and the
    /// [`IndirectedBlocks`] obtained after adding `blocks` to `self`.
    ///
    /// In the resulting symmetric difference, all the blocks starting at the given `offset` (included) are considered as modified.
    ///
    /// If the `offset` is [`None`], only the added blocks are considered as modified.
    #[allow(clippy::too_many_lines)]
    #[must_use]
    pub(crate) fn append_blocks_with_difference(&self, blocks: &[u32], offset: Option<u32>) -> (Self, SymmetricDifference<DBPC>) {
        let (last_indirection, last_index) = match offset {
            None => {
                let Some((_, (last_indirection, last_index))) = self.last_data_block_allocated() else {
                    return (self.clone(), SymmetricDifference {
                        blocks_per_indirection: self.blocks_per_indirection,
                        direct_blocks: (0, Vec::new()),
                        singly_indirected_blocks: (0, (0, Vec::new())),
                        doubly_indirected_blocks: (0, (0, Vec::new())),
                        triply_indirected_blocks: (0, (0, Vec::new())),
                    });
                };
                (last_indirection, last_index + 1)
            },
            Some(offset) => match Self::block_at_offset_remainging_in_indirection(offset, self.blocks_per_indirection) {
                Some(indirection_offset) => indirection_offset,
                None => {
                    return (self.clone(), SymmetricDifference {
                        blocks_per_indirection: self.blocks_per_indirection,
                        direct_blocks: (0, Vec::new()),
                        singly_indirected_blocks: (0, (0, Vec::new())),
                        doubly_indirected_blocks: (0, (0, Vec::new())),
                        triply_indirected_blocks: (0, (0, Vec::new())),
                    });
                },
            },
        };

        let index = u32_to_usize(last_index);

        let mut indirection = self.clone();
        indirection.append_blocks(blocks);
        let untouched_indirection = indirection.clone();

        match last_indirection {
            Indirection::Direct => {
                indirection.direct_blocks = indirection.direct_blocks.get(index..).map(<[u32]>::to_vec).unwrap_or_default();
            },
            Indirection::Simple => {
                indirection.direct_blocks = Vec::new();
                indirection.singly_indirected_blocks.1 =
                    indirection.singly_indirected_blocks.1.get(index..).map(<[_]>::to_vec).unwrap_or_default();

                if indirection.singly_indirected_blocks.1.is_empty() {
                    indirection.singly_indirected_blocks.0 = 0;
                }
            },
            Indirection::Double => {
                indirection.direct_blocks = Vec::new();
                indirection.singly_indirected_blocks = (0, Vec::new());

                indirection.doubly_indirected_blocks.1 = indirection
                    .doubly_indirected_blocks
                    .1
                    .get(u32_to_usize(last_index / self.blocks_per_indirection)..)
                    .map(<[_]>::to_vec)
                    .unwrap_or_default();

                if let Some(simple_block_indirection) = indirection.doubly_indirected_blocks.1.first_mut() {
                    simple_block_indirection.1 = simple_block_indirection
                        .1
                        .get(u32_to_usize(last_index % self.blocks_per_indirection)..)
                        .map(<[_]>::to_vec)
                        .unwrap_or_default();
                    if simple_block_indirection.1.is_empty() {
                        simple_block_indirection.0 = 0;
                    }
                } else {
                    indirection.doubly_indirected_blocks.0 = 0;
                };
            },
            Indirection::Triple => {
                indirection.direct_blocks = Vec::new();
                indirection.singly_indirected_blocks = (0, Vec::new());
                indirection.doubly_indirected_blocks = (0, Vec::new());

                indirection.triply_indirected_blocks.1 = indirection
                    .triply_indirected_blocks
                    .1
                    .get(u32_to_usize(last_index / (self.blocks_per_indirection * self.blocks_per_indirection))..)
                    .map(<[_]>::to_vec)
                    .unwrap_or_default();

                if let Some(double_block_indirection) = indirection.triply_indirected_blocks.1.first_mut() {
                    if last_index % (self.blocks_per_indirection * self.blocks_per_indirection) == 0 {
                        *double_block_indirection = (0, Vec::new());
                    } else {
                        double_block_indirection.1 = double_block_indirection
                            .1
                            .get(
                                u32_to_usize(
                                    (last_index % (self.blocks_per_indirection * self.blocks_per_indirection))
                                        / self.blocks_per_indirection,
                                )..,
                            )
                            .map(<[_]>::to_vec)
                            .unwrap_or_default();

                        if let Some(simple_block_indirection) = double_block_indirection.1.first_mut() {
                            if last_index % self.blocks_per_indirection == 0 {
                                *simple_block_indirection = (0, Vec::new());
                            } else {
                                simple_block_indirection.1 = simple_block_indirection
                                    .1
                                    .get(u32_to_usize(last_index % self.blocks_per_indirection)..)
                                    .map(<[_]>::to_vec)
                                    .unwrap_or_default();
                            }
                        }
                    }
                }
            },
        }

        let starting_index = |indirect| if last_indirection == indirect { u32_to_usize(last_index) } else { 0 };

        (untouched_indirection, SymmetricDifference {
            blocks_per_indirection: indirection.blocks_per_indirection,
            direct_blocks: (starting_index(Indirection::Direct), indirection.direct_blocks),
            singly_indirected_blocks: (starting_index(Indirection::Simple), indirection.singly_indirected_blocks),
            doubly_indirected_blocks: (starting_index(Indirection::Double), indirection.doubly_indirected_blocks),
            triply_indirected_blocks: (starting_index(Indirection::Triple), indirection.triply_indirected_blocks),
        })
    }
}

/// Type alias representing direct blocks and their starting offset.
///
/// The value `(index, blocks)` thus represents a [`DirectBlocks`] without all the leading "0": the `index` is the offset of the
/// first element of `blocks` in the corresponding [`DirectBlocks`].
pub type DirectBlocksOffset = (usize, DirectBlocks);

/// Type alias representing a single indirection block and its starting offset.
///
/// See [`DirectBlocksOffset`] for more information.
#[allow(clippy::module_name_repetitions)]
pub type SimpleIndirectionOffset = (usize, SimpleIndirection);

/// Type alias representing a double indirection block and its starting offset.
///
/// See [`DirectBlocksOffset`] for more information.
#[allow(clippy::module_name_repetitions)]
pub type DoubleIndirectionOffset = (usize, DoubleIndirection);

/// Type alias representing a triple indirection block and its starting offset.
///
/// See [`DirectBlocksOffset`] for more information.
#[allow(clippy::module_name_repetitions)]
pub type TripleIndirectionOffset = (usize, TripleIndirection);

/// Represents the symmetric difference between two [`IndirectedBlocks`].
///
/// This can be very useful in the manipulation of [`IndirectedBlocks`] during the addition or the removal of some indirection
/// blocks.
#[derive(Debug, PartialEq, Eq)]
pub struct SymmetricDifference<const DBPC: u32> {
    /// Number of blocks contained in each indirection.
    ///
    /// In ext2 filesystems, this always should be equal to `superblock.block_size() / 4`.
    blocks_per_indirection: u32,

    /// The direct block numbers.
    direct_blocks: DirectBlocksOffset,

    /// The singly indirected block numbers.
    singly_indirected_blocks: SimpleIndirectionOffset,

    /// The doubly indirected block numbers.
    doubly_indirected_blocks: DoubleIndirectionOffset,

    /// The triply indirected block numbers.
    triply_indirected_blocks: TripleIndirectionOffset,
}

impl<const DBPC: u32> SymmetricDifference<DBPC> {
    /// Returns the list of all changed blocks of indirection.
    ///
    /// In each tuple, the first element is the index at which the first block contained in this indirection block begins, the
    /// second element is a tuple containing the indirection block and the list of the blocks contained in this indirection block.
    ///
    /// All blocks, **starting at the first non-zero block** to the end, should be considered as being changed.
    #[must_use]
    pub fn changed_indirected_blocks(&self) -> Vec<(usize, (u32, Vec<u32>))> {
        let blocks_per_indirection = u32_to_usize(self.blocks_per_indirection);

        let mut indirected_blocks = Vec::new();

        if self.singly_indirected_blocks.1.0 != 0 {
            indirected_blocks.push((
                self.singly_indirected_blocks.0,
                (self.singly_indirected_blocks.1.0, self.singly_indirected_blocks.1.1.clone()),
            ));
        }

        if self.doubly_indirected_blocks.1.0 != 0 {
            indirected_blocks.push((
                self.doubly_indirected_blocks.0 / blocks_per_indirection,
                (
                    self.doubly_indirected_blocks.1.0,
                    self.doubly_indirected_blocks.1.1.iter().map(|(block, _)| *block).collect_vec(),
                ),
            ));

            let indirect_block_iterator = &mut self.doubly_indirected_blocks.1.1.iter();

            if let Some((indirection_block, blocks)) = indirect_block_iterator.next() {
                indirected_blocks
                    .push((self.doubly_indirected_blocks.0 % blocks_per_indirection, (*indirection_block, blocks.clone())));
            }

            for (indirection_block, blocks) in indirect_block_iterator.by_ref() {
                indirected_blocks.push((0, (*indirection_block, blocks.clone())));
            }
        }

        if self.triply_indirected_blocks.1.0 != 0 {
            indirected_blocks.push((
                self.triply_indirected_blocks.0 / (blocks_per_indirection * blocks_per_indirection),
                (
                    self.triply_indirected_blocks.1.0,
                    self.triply_indirected_blocks.1.1.iter().map(|(block, _)| *block).collect_vec(),
                ),
            ));

            let triply_indirect_block_iterator = &mut self.triply_indirected_blocks.1.1.iter();

            if let Some((double_indirection_block, doubly_indirected_blocks)) = triply_indirect_block_iterator.next() {
                indirected_blocks.push((
                    (self.triply_indirected_blocks.0 % (blocks_per_indirection * blocks_per_indirection)) / blocks_per_indirection,
                    (*double_indirection_block, doubly_indirected_blocks.iter().map(|(block, _)| *block).collect_vec()),
                ));

                let doubly_indirect_block_iterator = &mut doubly_indirected_blocks.iter();

                if let Some((indirect_block, blocks)) = doubly_indirect_block_iterator.next() {
                    indirected_blocks
                        .push((self.triply_indirected_blocks.0 % blocks_per_indirection, (*indirect_block, blocks.clone())));
                }

                for (indirect_block, blocks) in doubly_indirect_block_iterator.by_ref() {
                    indirected_blocks.push((0, (*indirect_block, blocks.clone())));
                }
            }

            for (double_indirection_block, doubly_indirected_blocks) in triply_indirect_block_iterator {
                indirected_blocks
                    .push((0, (*double_indirection_block, doubly_indirected_blocks.iter().map(|(block, _)| *block).collect_vec())));

                for (indirection_block, blocks) in doubly_indirected_blocks {
                    indirected_blocks.push((0, (*indirection_block, blocks.clone())));
                }
            }
        }

        indirected_blocks
    }

    /// Returns the list of all changed data blocks.
    ///
    /// As a write is always on contiguous blocks (from [`IndirectedBlocks`] point of vue), those data blocks should also be
    /// considered as contiguous. Thus, a write should modify every block starting at the first one returned.
    #[must_use]
    pub fn changed_data_blocks(&self) -> Vec<u32> {
        let mut data_blocks = Vec::new();

        data_blocks.append(&mut self.direct_blocks.1.clone());

        data_blocks.append(&mut self.singly_indirected_blocks.1.1.clone());

        for (_, blocks) in &self.doubly_indirected_blocks.1.1 {
            data_blocks.append(&mut blocks.clone());
        }

        for (_, indirected_blocks) in &self.triply_indirected_blocks.1.1 {
            for (_, blocks) in indirected_blocks {
                data_blocks.append(&mut blocks.clone());
            }
        }

        data_blocks
    }
}

#[cfg(test)]
mod test {
    use alloc::vec;

    use super::{IndirectedBlocks, Indirection};
    use crate::fs::ext2::inode::DIRECT_BLOCK_POINTER_COUNT;
    use crate::fs::structures::indirection::SymmetricDifference;

    #[test]
    fn direct_indirection() {
        let indirected_blocks = IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(
            5,
            vec![1, 2, 3, 4, 5, 6, 7],
            (0, vec![]),
            (0, vec![]),
            (0, vec![]),
        );
        assert_eq!(indirected_blocks.block_at_offset(3), Some(4));
    }

    #[test]
    fn simple_indirection() {
        let indirected_blocks = IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(
            1_024,
            vec![1; 12],
            (1, vec![1, 1, 2, 1, 1]),
            (0, vec![]),
            (0, vec![]),
        );
        assert_eq!(indirected_blocks.block_at_offset(14), Some(2));
    }

    #[test]
    fn double_indirection() {
        let indirected_blocks = IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(
            5,
            vec![1; 12],
            (1, vec![1; 5]),
            (1, vec![(1, vec![1; 5]), (1, vec![1; 5]), (1, vec![1, 2, 1, 1, 1]), (1, vec![1; 3])]),
            (0, vec![]),
        );
        assert_eq!(indirected_blocks.block_at_offset(3), Some(1));
        assert_eq!(indirected_blocks.block_at_offset(27), Some(1));
        assert_eq!(indirected_blocks.block_at_offset(28), Some(2));
        assert_eq!(indirected_blocks.block_at_offset(1_000), None);
    }

    #[test]
    fn triple_indirection() {
        let indirected_blocks = IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(
            5,
            vec![1; 12],
            (1, vec![1; 5]),
            (1, vec![(1, vec![1; 5]), (1, vec![1; 5]), (1, vec![1; 5]), (1, vec![1; 5]), (1, vec![1; 5])]),
            (1, vec![
                (1, vec![(1, vec![1; 5]), (1, vec![1; 5]), (1, vec![1; 5]), (1, vec![1; 5]), (1, vec![1; 5])]),
                (1, vec![(1, vec![1; 5]), (1, vec![1; 5]), (1, vec![1; 5]), (1, vec![1; 5]), (1, vec![1; 5])]),
                (1, vec![(1, vec![1; 5]), (1, vec![2, 1, 1, 1, 1]), (1, vec![1; 5]), (1, vec![1; 5]), (1, vec![1; 5])]),
            ]),
        );
        assert_eq!(indirected_blocks.block_at_offset(97), Some(2));
    }

    #[test]
    fn last_data_block_allocated() {
        let indirected_blocks = IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(
            5,
            vec![1, 2, 3, 4, 5, 6, 7],
            (0, vec![]),
            (0, vec![]),
            (0, vec![]),
        );
        assert_eq!(indirected_blocks.last_data_block_allocated(), Some((7, (Indirection::Direct, 6))));

        let indirected_blocks =
            IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(1_024, vec![0; 12], (0, vec![0, 0, 1]), (0, vec![]), (0, vec![]));
        assert_eq!(indirected_blocks.last_data_block_allocated(), Some((1, (Indirection::Simple, 2))));

        let indirected_blocks = IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(
            5,
            vec![0; 12],
            (0, vec![0; 5]),
            (0, vec![(0, vec![0; 5]), (0, vec![0; 5]), (0, vec![0, 1])]),
            (0, vec![]),
        );
        assert_eq!(indirected_blocks.last_data_block_allocated(), Some((1, (Indirection::Double, 11))));

        let indirected_blocks = IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(
            5,
            vec![0; 12],
            (0, vec![0; 5]),
            (0, vec![(0, vec![0; 5]), (0, vec![0; 5]), (0, vec![0; 5]), (0, vec![0; 5]), (0, vec![0; 5])]),
            (0, vec![
                (0, vec![(0, vec![0; 5]), (0, vec![0; 5]), (0, vec![0; 5]), (0, vec![0; 5]), (0, vec![0; 5])]),
                (0, vec![(0, vec![0; 5]), (0, vec![0; 5]), (0, vec![0; 5]), (0, vec![0; 5]), (0, vec![0; 5])]),
                (1, vec![(0, vec![0; 5]), (0, vec![1])]),
            ]),
        );
        assert_eq!(indirected_blocks.last_data_block_allocated(), Some((1, (Indirection::Triple, 55))));
    }

    #[test]
    fn block_counts() {
        let indirected_blocks = IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(
            5,
            vec![1, 2, 3, 4, 5, 6, 7],
            (0, vec![]),
            (0, vec![]),
            (0, vec![]),
        );
        assert_eq!(indirected_blocks.data_block_count(), 7);
        assert_eq!(indirected_blocks.indirection_block_count(), 0);
        assert_eq!(IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::necessary_indirection_block_count(7, 5), 0);

        let indirected_blocks =
            IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(1_024, vec![1; 12], (1, vec![1, 1, 1]), (0, vec![]), (0, vec![]));
        assert_eq!(indirected_blocks.data_block_count(), 15);
        assert_eq!(indirected_blocks.indirection_block_count(), 1);
        assert_eq!(IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::necessary_indirection_block_count(15, 5), 1);

        let indirected_blocks = IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(
            5,
            vec![1; 12],
            (1, vec![1; 5]),
            (1, vec![(1, vec![1; 5]), (1, vec![1; 5]), (1, vec![1, 1])]),
            (0, vec![]),
        );
        assert_eq!(indirected_blocks.data_block_count(), 29);
        assert_eq!(indirected_blocks.indirection_block_count(), 5);
        assert_eq!(IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::necessary_indirection_block_count(29, 5), 5);

        let indirected_blocks = IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(
            5,
            vec![1; 12],
            (1, vec![1; 5]),
            (1, vec![(1, vec![1; 5]), (1, vec![1; 5]), (1, vec![1; 5]), (1, vec![1; 5]), (1, vec![1; 5])]),
            (1, vec![
                (1, vec![(1, vec![1; 5]), (1, vec![1; 5]), (1, vec![1; 5]), (1, vec![1; 5]), (1, vec![1; 5])]),
                (1, vec![(1, vec![1; 5]), (1, vec![1; 5]), (1, vec![1; 5]), (1, vec![1; 5]), (1, vec![1; 5])]),
                (1, vec![(1, vec![1; 5]), (1, vec![1; 5])]),
            ]),
        );
        assert_eq!(indirected_blocks.data_block_count(), 102);
        assert_eq!(indirected_blocks.indirection_block_count(), 23);
        assert_eq!(IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::necessary_indirection_block_count(102, 5), 23);

        let indirected_blocks = IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(
            5,
            vec![1; 12],
            (1, vec![1; 5]),
            (1, vec![(1, vec![5; 1]); 5]),
            (1, vec![(1, vec![(1, vec![1; 5]); 5]); 5]),
        );
        assert_eq!(indirected_blocks.data_block_count(), 167);
        assert_eq!(indirected_blocks.indirection_block_count(), 38);
    }

    #[test]
    fn append_blocks() {
        let mut indirected_blocks = IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(
            5,
            vec![1, 2, 3, 4, 5, 6, 7],
            (0, vec![]),
            (0, vec![]),
            (0, vec![]),
        );
        let indirected_blocks_after_append_1 = IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(
            5,
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
            (0, vec![]),
            (0, vec![]),
            (0, vec![]),
        );
        let indirected_blocks_after_append_2 = IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(
            5,
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
            (13, vec![14]),
            (0, vec![]),
            (0, vec![]),
        );

        indirected_blocks.append_blocks(&[8, 9, 10, 11]);
        assert_eq!(indirected_blocks, indirected_blocks_after_append_1);
        indirected_blocks.append_blocks(&[12, 13, 14]);
        assert_eq!(indirected_blocks, indirected_blocks_after_append_2);

        let mut indirected_blocks =
            IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(5, vec![1; 12], (1, vec![1, 1, 1]), (0, vec![]), (0, vec![]));
        let indirected_blocks_after_append_1 =
            IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(5, vec![1; 12], (1, vec![1, 1, 1, 2, 2]), (2, vec![]), (0, vec![]));
        let indirected_blocks_after_append_2 = IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(
            5,
            vec![1; 12],
            (1, vec![1, 1, 1, 2, 2]),
            (2, vec![(3, vec![3; 5]), (3, vec![])]),
            (0, vec![]),
        );

        indirected_blocks.append_blocks(&[2; 3]);
        assert_eq!(indirected_blocks, indirected_blocks_after_append_1);
        indirected_blocks.append_blocks(&[3; 7]);
        assert_eq!(indirected_blocks, indirected_blocks_after_append_2);

        let mut indirected_blocks = IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(
            5,
            vec![1; 12],
            (1, vec![1; 5]),
            (1, vec![(1, vec![1; 5]), (1, vec![1; 5]), (1, vec![1, 1])]),
            (0, vec![]),
        );
        let indirected_blocks_after_append = IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(
            5,
            vec![1; 12],
            (1, vec![1; 5]),
            (1, vec![(1, vec![1; 5]), (1, vec![1; 5]), (1, vec![1, 1, 2, 2, 2]), (2, vec![2; 5]), (2, vec![2; 5])]),
            (2, vec![(2, vec![(2, vec![2; 3])])]),
        );

        indirected_blocks.append_blocks(&[2; 21]);
        assert_eq!(indirected_blocks, indirected_blocks_after_append);
    }

    #[test]
    fn flatten() {
        let indirected_blocks = IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(
            5,
            vec![1, 2, 3, 4, 5, 6, 7],
            (0, vec![]),
            (0, vec![]),
            (0, vec![]),
        );
        assert_eq!(indirected_blocks.flatten_data_blocks_with_indirection(), vec![
            (1, (Indirection::Direct, 0)),
            (2, (Indirection::Direct, 1)),
            (3, (Indirection::Direct, 2)),
            (4, (Indirection::Direct, 3)),
            (5, (Indirection::Direct, 4)),
            (6, (Indirection::Direct, 5)),
            (7, (Indirection::Direct, 6))
        ]);

        let indirected_blocks =
            IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(5, vec![1; 12], (2, vec![3, 3, 3]), (0, vec![]), (0, vec![]));
        assert_eq!(indirected_blocks.flatten_data_blocks_with_indirection(), vec![
            (1, (Indirection::Direct, 0)),
            (1, (Indirection::Direct, 1)),
            (1, (Indirection::Direct, 2)),
            (1, (Indirection::Direct, 3)),
            (1, (Indirection::Direct, 4)),
            (1, (Indirection::Direct, 5)),
            (1, (Indirection::Direct, 6)),
            (1, (Indirection::Direct, 7)),
            (1, (Indirection::Direct, 8)),
            (1, (Indirection::Direct, 9)),
            (1, (Indirection::Direct, 10)),
            (1, (Indirection::Direct, 11)),
            (3, (Indirection::Simple, 0)),
            (3, (Indirection::Simple, 1)),
            (3, (Indirection::Simple, 2))
        ]);
    }

    #[test]
    fn append_blocks_with_difference() {
        let indirected_blocks = IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(
            5,
            vec![1, 2, 3, 4, 5, 6, 7],
            (0, vec![]),
            (0, vec![]),
            (0, vec![]),
        );
        assert_eq!(indirected_blocks.append_blocks_with_difference(&[8, 9, 10], None).1, SymmetricDifference {
            blocks_per_indirection: 5,
            direct_blocks: (7, vec![8, 9, 10]),
            singly_indirected_blocks: (0, (0, vec![])),
            doubly_indirected_blocks: (0, (0, vec![])),
            triply_indirected_blocks: (0, (0, vec![]))
        });

        let indirected_blocks =
            IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(5, vec![1; 12], (1, vec![1, 1, 1]), (0, vec![]), (0, vec![]));

        assert_eq!(indirected_blocks.append_blocks_with_difference(&[2, 3, 4, 5], None).1, SymmetricDifference {
            blocks_per_indirection: 5,
            direct_blocks: (0, vec![]),
            singly_indirected_blocks: (3, (1, vec![2, 3])),
            doubly_indirected_blocks: (0, (4, vec![(5, vec![])])),
            triply_indirected_blocks: (0, (0, vec![]))
        });

        let indirected_blocks = IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(
            5,
            vec![1; 12],
            (1, vec![1, 1, 1, 2, 2]),
            (2, vec![(3, vec![3; 5]), (3, vec![3])]),
            (0, vec![]),
        );
        assert_eq!(indirected_blocks.append_blocks_with_difference(&[4, 5, 6, 7, 8, 9, 10], None).1, SymmetricDifference {
            blocks_per_indirection: 5,
            direct_blocks: (0, vec![]),
            singly_indirected_blocks: (0, (0, vec![])),
            doubly_indirected_blocks: (6, (2, vec![(3, vec![4, 5, 6, 7]), (8, vec![9, 10])])),
            triply_indirected_blocks: (0, (0, vec![]))
        });

        let indirected_blocks = IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(
            5,
            vec![1; 12],
            (1, vec![1; 5]),
            (1, vec![(1, vec![1; 5]), (1, vec![1; 5]), (1, vec![1, 1, 2, 2, 2]), (2, vec![2; 5]), (2, vec![2; 5])]),
            (2, vec![(2, vec![(2, vec![2; 5]), (2, vec![2; 3])])]),
        );
        assert_eq!(indirected_blocks.append_blocks_with_difference(&[3, 4, 5, 6, 7, 8], None).1, SymmetricDifference {
            blocks_per_indirection: 5,
            direct_blocks: (0, vec![]),
            singly_indirected_blocks: (0, (0, vec![])),
            doubly_indirected_blocks: (0, (0, vec![])),
            triply_indirected_blocks: (8, (2, vec![(2, vec![(2, vec![3, 4]), (5, vec![6, 7, 8])])]))
        });

        let indirected_blocks = IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(
            5,
            vec![1; 12],
            (1, vec![1, 1, 1, 2, 2]),
            (2, vec![(3, vec![3; 5]), (3, vec![3])]),
            (0, vec![]),
        );
        assert_eq!(indirected_blocks.append_blocks_with_difference(&[4, 5, 6, 7, 8, 9, 10], Some(0)).1, SymmetricDifference {
            blocks_per_indirection: 5,
            direct_blocks: (0, vec![1; 12]),
            singly_indirected_blocks: (0, (1, vec![1, 1, 1, 2, 2])),
            doubly_indirected_blocks: (0, (2, vec![(3, vec![3; 5]), (3, vec![3, 4, 5, 6, 7]), (8, vec![9, 10])])),
            triply_indirected_blocks: (0, (0, vec![]))
        });

        let indirected_blocks = IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(
            5,
            vec![1; 12],
            (1, vec![1, 1, 1, 2, 2]),
            (2, vec![(3, vec![3; 5]), (3, vec![3])]),
            (0, vec![]),
        );
        assert_eq!(indirected_blocks.append_blocks_with_difference(&[4, 5, 6, 7, 8, 9, 10], Some(13)).1, SymmetricDifference {
            blocks_per_indirection: 5,
            direct_blocks: (0, vec![]),
            singly_indirected_blocks: (1, (1, vec![1, 1, 2, 2])),
            doubly_indirected_blocks: (0, (2, vec![(3, vec![3; 5]), (3, vec![3, 4, 5, 6, 7]), (8, vec![9, 10])])),
            triply_indirected_blocks: (0, (0, vec![]))
        });

        let indirected_blocks = IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(
            5,
            vec![1; 12],
            (1, vec![1; 5]),
            (1, vec![(1, vec![1; 5]), (1, vec![1; 5]), (1, vec![1, 1, 2, 2, 2]), (2, vec![2; 5]), (2, vec![2; 5])]),
            (2, vec![(2, vec![(2, vec![2; 5]), (2, vec![2; 5]), (3, vec![3; 4])])]),
        );
        assert_eq!(indirected_blocks.append_blocks_with_difference(&[4, 5, 6, 7, 8, 9, 10], Some(55)).1, SymmetricDifference {
            blocks_per_indirection: 5,
            direct_blocks: (0, vec![]),
            singly_indirected_blocks: (0, (0, vec![])),
            doubly_indirected_blocks: (0, (0, vec![])),
            triply_indirected_blocks: (13, (2, vec![(2, vec![(3, vec![3, 4]), (5, vec![6, 7, 8, 9, 10])])]))
        });
    }

    #[test]
    fn symmetric_difference_changes() {
        let symmetric_difference = SymmetricDifference::<DIRECT_BLOCK_POINTER_COUNT> {
            blocks_per_indirection: 5,
            direct_blocks: (5, vec![1, 2, 3, 4]),
            singly_indirected_blocks: (0, (0, vec![])),
            doubly_indirected_blocks: (0, (0, vec![])),
            triply_indirected_blocks: (0, (0, vec![])),
        };
        assert_eq!(symmetric_difference.changed_indirected_blocks(), vec![]);
        assert_eq!(symmetric_difference.changed_data_blocks(), vec![1, 2, 3, 4]);

        let symmetric_difference = SymmetricDifference::<DIRECT_BLOCK_POINTER_COUNT> {
            blocks_per_indirection: 5,
            direct_blocks: (10, vec![1, 2]),
            singly_indirected_blocks: (0, (3, vec![4, 5])),
            doubly_indirected_blocks: (0, (0, vec![])),
            triply_indirected_blocks: (0, (0, vec![])),
        };
        assert_eq!(symmetric_difference.changed_indirected_blocks(), vec![(0, (3, vec![4, 5]))]);
        assert_eq!(symmetric_difference.changed_data_blocks(), vec![1, 2, 4, 5]);

        let symmetric_difference = SymmetricDifference::<DIRECT_BLOCK_POINTER_COUNT> {
            blocks_per_indirection: 5,
            direct_blocks: (0, vec![]),
            singly_indirected_blocks: (3, (1, vec![2, 3])),
            doubly_indirected_blocks: (0, (4, vec![(5, vec![6, 7, 8])])),
            triply_indirected_blocks: (0, (0, vec![])),
        };
        assert_eq!(symmetric_difference.changed_indirected_blocks(), vec![
            (3, (1, vec![2, 3])),
            (0, (4, vec![5])),
            (0, (5, vec![6, 7, 8]))
        ]);
        assert_eq!(symmetric_difference.changed_data_blocks(), vec![2, 3, 6, 7, 8]);

        let symmetric_difference = SymmetricDifference::<DIRECT_BLOCK_POINTER_COUNT> {
            blocks_per_indirection: 5,
            direct_blocks: (0, vec![]),
            singly_indirected_blocks: (0, (0, vec![])),
            doubly_indirected_blocks: (19, (1, vec![(2, vec![4]), (5, vec![6, 7, 8, 9, 10])])),
            triply_indirected_blocks: (0, (11, vec![(12, vec![(13, vec![14, 15])])])),
        };
        assert_eq!(symmetric_difference.changed_indirected_blocks(), vec![
            (3, (1, vec![2, 5])),
            (4, (2, vec![4])),
            (0, (5, vec![6, 7, 8, 9, 10])),
            (0, (11, vec![12])),
            (0, (12, vec![13])),
            (0, (13, vec![14, 15])),
        ]);
        assert_eq!(symmetric_difference.changed_data_blocks(), vec![4, 6, 7, 8, 9, 10, 14, 15]);

        let symmetric_difference = SymmetricDifference::<DIRECT_BLOCK_POINTER_COUNT> {
            blocks_per_indirection: 5,
            direct_blocks: (0, vec![]),
            singly_indirected_blocks: (0, (0, vec![])),
            doubly_indirected_blocks: (0, (0, vec![])),
            triply_indirected_blocks: (
                43,
                (1, vec![(2, vec![(3, vec![4, 5]), (6, vec![7, 8, 9, 10, 11])]), (12, vec![(13, vec![14, 15, 16])])]),
            ),
        };
        assert_eq!(symmetric_difference.changed_indirected_blocks(), vec![
            (1, (1, vec![2, 12])),
            (3, (2, vec![3, 6])),
            (3, (3, vec![4, 5])),
            (0, (6, vec![7, 8, 9, 10, 11])),
            (0, (12, vec![13])),
            (0, (13, vec![14, 15, 16])),
        ]);
        assert_eq!(symmetric_difference.changed_data_blocks(), vec![4, 5, 7, 8, 9, 10, 11, 14, 15, 16]);
    }

    #[test]
    fn truncate_back() {
        let mut indirected_blocks = IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(
            5,
            vec![1, 2, 3, 4, 5, 6, 7],
            (0, vec![]),
            (0, vec![]),
            (0, vec![]),
        );
        let indirected_blocks_after_truncation =
            IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(5, vec![1, 2, 3], (0, vec![]), (0, vec![]), (0, vec![]));
        indirected_blocks.truncate_back_data_blocks(3);
        assert_eq!(indirected_blocks, indirected_blocks_after_truncation);

        let mut indirected_blocks =
            IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(5, vec![1; 12], (1, vec![1, 1, 1]), (0, vec![]), (0, vec![]));
        let indirected_blocks_after_truncation_1 =
            IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(5, vec![1; 12], (1, vec![1]), (0, vec![]), (0, vec![]));
        let indirected_blocks_after_truncation_2 =
            IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(5, vec![1; 3], (0, vec![]), (0, vec![]), (0, vec![]));
        indirected_blocks.truncate_back_data_blocks(13);
        assert_eq!(indirected_blocks, indirected_blocks_after_truncation_1);
        indirected_blocks.truncate_back_data_blocks(3);
        assert_eq!(indirected_blocks, indirected_blocks_after_truncation_2);

        let mut indirected_blocks = IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(
            5,
            vec![1; 12],
            (1, vec![1; 5]),
            (2, vec![(3, vec![3; 5]), (4, vec![4; 3])]),
            (0, vec![]),
        );
        let indirected_blocks_after_truncation_1 = IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(
            5,
            vec![1; 12],
            (1, vec![1; 5]),
            (2, vec![(3, vec![3; 3])]),
            (0, vec![]),
        );
        let indirected_blocks_after_truncation_2 =
            IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(5, vec![1; 12], (1, vec![1; 5]), (0, vec![]), (0, vec![]));
        indirected_blocks.truncate_back_data_blocks(20);
        assert_eq!(indirected_blocks, indirected_blocks_after_truncation_1);
        indirected_blocks.truncate_back_data_blocks(17);
        assert_eq!(indirected_blocks, indirected_blocks_after_truncation_2);

        let mut indirected_blocks = IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(
            5,
            vec![1; 12],
            (1, vec![1; 5]),
            (2, vec![(3, vec![3; 5]); 5]),
            (4, vec![(5, vec![(6, vec![6; 5]); 5]), (7, vec![(8, vec![8; 5]); 3])]),
        );
        let indirected_blocks_after_truncation_1 = IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(
            5,
            vec![1; 12],
            (1, vec![1; 5]),
            (2, vec![(3, vec![3; 5]); 5]),
            (4, vec![(5, vec![(6, vec![6; 5]); 3])]),
        );
        let indirected_blocks_after_truncation_2 = IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(
            5,
            vec![1; 12],
            (1, vec![1; 5]),
            (2, vec![(3, vec![3; 5]); 3]),
            (0, vec![]),
        );
        indirected_blocks.truncate_back_data_blocks(57);
        assert_eq!(indirected_blocks, indirected_blocks_after_truncation_1);
        indirected_blocks.truncate_back_data_blocks(32);
        assert_eq!(indirected_blocks, indirected_blocks_after_truncation_2);
    }

    #[test]
    fn truncate_front() {
        let indirected_blocks = IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(
            5,
            vec![1; 12],
            (1, vec![1; 5]),
            (2, vec![(3, vec![3; 5]); 5]),
            (4, vec![(5, vec![(6, vec![6; 5]); 5]), (7, vec![(8, vec![8; 5]); 3])]),
        );
        assert_eq!(indirected_blocks.truncate_front_data_blocks(5), SymmetricDifference {
            blocks_per_indirection: 5,
            direct_blocks: (5, vec![1; 7]),
            singly_indirected_blocks: (0, (1, vec![1; 5])),
            doubly_indirected_blocks: (0, (2, vec![(3, vec![3; 5]); 5])),
            triply_indirected_blocks: (0, (4, vec![(5, vec![(6, vec![6; 5]); 5]), (7, vec![(8, vec![8; 5]); 3])]))
        });

        let indirected_blocks = IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(
            5,
            vec![1; 12],
            (1, vec![1; 5]),
            (2, vec![(3, vec![3; 5]); 5]),
            (4, vec![(5, vec![(6, vec![6; 5]); 5]), (7, vec![(8, vec![8; 5]); 3])]),
        );
        assert_eq!(indirected_blocks.truncate_front_data_blocks(14), SymmetricDifference {
            blocks_per_indirection: 5,
            direct_blocks: (0, vec![]),
            singly_indirected_blocks: (2, (1, vec![1; 3])),
            doubly_indirected_blocks: (0, (2, vec![(3, vec![3; 5]); 5])),
            triply_indirected_blocks: (0, (4, vec![(5, vec![(6, vec![6; 5]); 5]), (7, vec![(8, vec![8; 5]); 3])]))
        });

        let indirected_blocks = IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(
            5,
            vec![1; 12],
            (1, vec![1; 5]),
            (2, vec![(3, vec![3; 5]); 5]),
            (4, vec![(5, vec![(6, vec![6; 5]); 5]), (7, vec![(8, vec![8; 5]); 3])]),
        );
        assert_eq!(indirected_blocks.truncate_front_data_blocks(26), SymmetricDifference {
            blocks_per_indirection: 5,
            direct_blocks: (0, vec![]),
            singly_indirected_blocks: (0, (0, vec![])),
            doubly_indirected_blocks: (9, (2, vec![(3, vec![3]), (3, vec![3; 5]), (3, vec![3; 5]), (3, vec![3; 5])])),
            triply_indirected_blocks: (0, (4, vec![(5, vec![(6, vec![6; 5]); 5]), (7, vec![(8, vec![8; 5]); 3])]))
        });

        let indirected_blocks = IndirectedBlocks::<DIRECT_BLOCK_POINTER_COUNT>::new(
            5,
            vec![1; 12],
            (1, vec![1; 5]),
            (2, vec![(3, vec![3; 5]); 5]),
            (4, vec![(5, vec![(6, vec![6; 5]); 5]), (7, vec![(8, vec![8; 5]); 3])]),
        );
        assert_eq!(indirected_blocks.truncate_front_data_blocks(81), SymmetricDifference {
            blocks_per_indirection: 5,
            direct_blocks: (0, vec![]),
            singly_indirected_blocks: (0, (0, vec![])),
            doubly_indirected_blocks: (0, (0, vec![])),
            triply_indirected_blocks: (39, (4, vec![(7, vec![(8, vec![8])])]))
        });
    }
}
