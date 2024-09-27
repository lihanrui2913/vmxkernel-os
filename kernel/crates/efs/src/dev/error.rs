//! Errors related to device manipulation.

use derive_more::derive::{Display, Error};

/// Enumeration of possible errors encountered with device's manipulation.
#[allow(clippy::module_name_repetitions)]
#[derive(Debug, PartialEq, Eq, Display, Error)]
#[display("Device Error: {_variant}")]
pub enum DevError {
    /// The given `structure` has a `value` not between the given bounds.
    #[display(
        "Out of Bounds: the {structure} has a value {value} not between the lower bound {lower_bound} and the upper bound {upper_bound}"
    )]
    OutOfBounds {
        /// Name of the bounded structure.
        structure: &'static str,

        /// Given value.
        value: i128,

        /// Lower bound for the structure.
        lower_bound: i128,

        /// Upper bound for the structure.
        upper_bound: i128,
    },

    /// An error returned when an operation could not be completed because an “end of file” was reached prematurely.
    ///
    /// This typically means that an operation could only succeed if it read a particular number of bytes but only a smaller number
    /// of bytes could be read.
    #[display("Unexpected End of File: an operation could not be completed because an \"end of file\" was reached prematurely")]
    UnexpectedEof,

    /// An error returned when an operation could not be completed because a call to [`write`](crate::io::Write::write) returned
    /// `Ok(0)`.
    #[display("Write Zero: An error returned when an operation could not be completed because a call to write returned Ok(0)")]
    WriteZero,
}
