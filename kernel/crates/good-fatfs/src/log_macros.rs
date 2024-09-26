//! This module offers a convenient way to enable only a subset of logging levels
//! for just this `fatfs` crate only without changing the logging levels
//! of other crates in a given project.

use log::LevelFilter;

#[cfg(feature = "log_level_trace")]
pub const MAX_LOG_LEVEL: LevelFilter = LevelFilter::Trace;

#[cfg(all(not(feature = "log_level_trace"), feature = "log_level_debug",))]
pub const MAX_LOG_LEVEL: LevelFilter = LevelFilter::Debug;

#[cfg(all(
    not(feature = "log_level_trace"),
    not(feature = "log_level_debug"),
    feature = "log_level_info",
))]
pub const MAX_LOG_LEVEL: LevelFilter = LevelFilter::Info;

#[cfg(all(
    not(feature = "log_level_trace"),
    not(feature = "log_level_debug"),
    not(feature = "log_level_info"),
    feature = "log_level_warn",
))]
pub const MAX_LOG_LEVEL: LevelFilter = LevelFilter::Warn;

#[cfg(all(
    not(feature = "log_level_trace"),
    not(feature = "log_level_debug"),
    not(feature = "log_level_info"),
    not(feature = "log_level_warn"),
    feature = "log_level_error",
))]
pub const MAX_LOG_LEVEL: LevelFilter = LevelFilter::Error;

#[cfg(all(
    not(feature = "log_level_trace"),
    not(feature = "log_level_debug"),
    not(feature = "log_level_info"),
    not(feature = "log_level_warn"),
    not(feature = "log_level_error"),
))]
pub const MAX_LOG_LEVEL: LevelFilter = LevelFilter::Off;
