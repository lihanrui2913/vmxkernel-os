//! Interface to manipulate caches.
//!
//! The cache structure used here is a simple balanced binary search tree.

use alloc::collections::BTreeMap;

use derive_more::{Deref, DerefMut};
use spin::{Mutex, RwLock};

/// State of a stored value.
///
/// Can be useful if a value changed but the new value is hard to compute and not needed.
#[derive(Debug, Clone, Copy)]
pub enum State<V> {
    /// The value is valid and can be safely read.
    Valid(V),

    /// The value is invalid and should not be read.
    Invalid,
}

impl<T> From<Option<T>> for State<T> {
    fn from(value: Option<T>) -> Self {
        value.map_or_else(|| Self::Invalid, |v| Self::Valid(v))
    }
}

impl<V> From<State<V>> for Option<V> {
    fn from(value: State<V>) -> Self {
        match value {
            State::Valid(v) => Some(v),
            State::Invalid => None,
        }
    }
}

/// Generic cache structure.
#[derive(Debug, Default, Deref, DerefMut)]
pub struct Cache<K, V>(pub Mutex<BTreeMap<K, State<RwLock<V>>>>);

impl<K: Ord, V> Cache<K, V> {
    /// Returns a new empty cache.
    #[must_use]
    pub const fn new() -> Self {
        Self(Mutex::new(BTreeMap::new()))
    }

    /// Inserts a key-value pair into the map.
    pub fn insert(&self, key: K, value: V) {
        self.0.lock().insert(key, State::Valid(RwLock::new(value)));
    }
}

impl<K: Ord, V: Copy> Cache<K, V> {
    /// Returns a copy of the value.
    pub fn get_copy(&self, key: &K) -> Option<V> {
        self.0.lock().get(key).and_then(|state| match state {
            State::Valid(value) => Some(*value.read()),
            State::Invalid => None,
        })
    }
}
