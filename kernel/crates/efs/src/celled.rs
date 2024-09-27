//! Interface to use celled objects.
//!
//! It provides an interface to contenerize objects with the guarantee that the `clone` method is not expansive and the new celled
//! object point to the same initial one;

use alloc::sync::Arc;

use derive_more::{Deref, DerefMut};
use spin::Mutex;

/// Type alias for celled objects.
#[derive(Debug, Deref, DerefMut)]
pub struct Celled<T>(Arc<Mutex<T>>);

impl<T> Clone for Celled<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T> Celled<T> {
    /// Creates a new celled object.
    pub fn new(obj: T) -> Self {
        Self(Arc::new(Mutex::new(obj)))
    }
}
