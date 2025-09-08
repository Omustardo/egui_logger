use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

use crate::types::LogLevel;

/// The "internal" representation of a log line.
/// These are created and stored within calls like [`Self::log_info`] and aren't expected to
/// be used often. There are two issues with expected usage:
/// * timestamp: when the LogRecord is created may not be when you want to insert it into
///   the chat log.
/// * categories must be a String here. This is an internal implementation detail. It supports
///   any type that can be turned into a String. This is normally expected to be an Enum, so
///   if you are creating LogRecords yourself, you should probably use [`Self::get_log_record`]
///   which does the T->String conversion for you.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LogRecord {
    pub timestamp: DateTime<Local>,
    pub level: LogLevel,
    pub categories: Vec<String>,
    pub message: String,
}
