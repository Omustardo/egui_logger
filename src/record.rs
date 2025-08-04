use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

use crate::types::LogLevel;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LogRecord {
    pub timestamp: DateTime<Local>,
    pub level: LogLevel,
    pub categories: Vec<String>,
    pub message: String,
}