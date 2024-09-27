//! Bitmap manipulation for devices.
//!
//! Bitmap are frequently used data types, so this is a general interface to manipulate them.

use alloc::vec::Vec;
use core::fmt::Debug;
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};

use crate::arch::u32_to_usize;
use crate::celled::Celled;
use crate::dev::Device;
use crate::dev::sector::Address;
use crate::error::Error;

/// Generic bitmap structure.
///
/// It can handles any [`Copy`] structure directly written onto a [`Device`].
///
/// See [the Wikipedia page](https://en.wikipedia.org/wiki/Bit_array) for more general informations.
pub struct Bitmap<T: Copy, FSE: core::error::Error, Dev: Device<T, FSE>> {
    /// Device containing the bitmap.
    device: Celled<Dev>,

    /// Inner elements.
    inner: Vec<T>,

    /// Starting address of the bitmap on the device.
    starting_addr: Address,

    /// Length of the bitmap.
    length: usize,

    /// Phantom data to use the `E` generic.
    phantom: PhantomData<FSE>,
}

impl<FSE: core::error::Error, T: Copy + Debug, Dev: Device<T, FSE>> Debug for Bitmap<T, FSE, Dev> {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        fmt.debug_struct("Bitmap")
            .field("inner", &self.inner)
            .field("starting_addr", &self.starting_addr)
            .field("length", &self.length)
            .field("phantom", &self.phantom)
            .finish()
    }
}

impl<FSE: core::error::Error, T: Copy, Dev: Device<T, FSE>> Bitmap<T, FSE, Dev> {
    /// Creates a new [`Bitmap`] instance from the device on which it is located, its starting address on the device and its length.
    ///
    /// # Errors
    ///
    /// Returns an [`Error::Device`] if the device cannot be read.
    pub fn new(celled_device: Celled<Dev>, starting_addr: Address, length: usize) -> Result<Self, Error<FSE>> {
        let inner = celled_device.lock().slice(starting_addr..(starting_addr + length))?.to_vec();
        Ok(Self {
            device: celled_device,
            inner,
            starting_addr,
            length,
            phantom: PhantomData,
        })
    }

    /// Returns the length of the bitmap.
    #[must_use]
    pub const fn length(&self) -> usize {
        self.length
    }

    /// Returns the starting address of the bitmap.
    #[must_use]
    pub const fn starting_address(&self) -> Address {
        self.starting_addr
    }

    /// Writes back the current state of the bitmap onto the device.
    ///
    /// # Errors
    ///
    /// Returns an [`Error::Device`] if the device cannot be written.
    pub fn write_back(&mut self) -> Result<(), Error<FSE>> {
        let mut device = self.device.lock();
        let mut slice = device.slice(self.starting_addr..(self.starting_addr + self.length))?;
        slice.clone_from_slice(&self.inner);
        let commit = slice.commit();
        device.commit(commit)?;

        Ok(())
    }

    /// Finds the first elements `el` such that the sum of all `count(el)` is greater than or equal to `n`.
    ///
    /// Returns the indices and the value of those elements, keeping only the ones satisfying `count(el) > 0`.
    ///
    /// If the sum of all `count(el)` is lesser than `n`, returns all the elements `el` such that `count(el) > 0`.
    pub fn find_to_count<F: Fn(&T) -> usize>(&self, n: usize, count: F) -> Vec<(usize, T)> {
        let mut counter = 0_usize;
        let mut element_taken = Vec::new();

        for (index, element) in self.inner.iter().enumerate() {
            let element_count = count(element);
            if element_count > 0 {
                counter += element_count;
                element_taken.push((index, *element));
                if counter >= n {
                    return element_taken;
                }
            }
        }

        element_taken
    }
}

impl<FSE: core::error::Error, Dev: Device<u8, FSE>> Bitmap<u8, FSE, Dev> {
    /// Specialization of [`find_to_count`](Bitmap::find_to_count) to find the first bytes such that the sum of set bits is at least
    /// `n`.
    #[must_use]
    pub fn find_n_set_bits(&self, n: usize) -> Vec<(usize, u8)> {
        self.find_to_count(n, |byte| {
            let mut count = byte - ((byte >> 1_u8) & 0x55);
            count = (count & 0x33) + ((count >> 2_u8) & 0x33);
            count = (count + (count >> 4_u8)) & 0x0F;
            u32_to_usize(count.into())
        })
    }

    /// Specialization of [`find_to_count`](Bitmap::find_to_count) to find the first bytes such that the sum of unset bits is at
    /// least `n`.
    #[must_use]
    pub fn find_n_unset_bits(&self, n: usize) -> Vec<(usize, u8)> {
        self.find_to_count(n, |byte| {
            let mut count = byte - ((byte >> 1_u8) & 0x55);
            count = (count & 0x33) + ((count >> 2_u8) & 0x33);
            count = (count + (count >> 4_u8)) & 0x0F;
            u32_to_usize(8_u32 - Into::<u32>::into(count))
        })
    }
}

impl<FSE: core::error::Error, T: Copy, Dev: Device<T, FSE>> IntoIterator for Bitmap<T, FSE, Dev> {
    type IntoIter = <Vec<T> as IntoIterator>::IntoIter;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<FSE: core::error::Error, T: Copy, Dev: Device<T, FSE>> Deref for Bitmap<T, FSE, Dev> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<FSE: core::error::Error, T: Copy, Dev: Device<T, FSE>> DerefMut for Bitmap<T, FSE, Dev> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
