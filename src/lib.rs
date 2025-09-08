#![doc = include_str!("../README.md")]

mod categories;
mod logger;
mod record;
mod tests;
mod types;
mod ui;
mod utils;

// Re-export public API
pub use categories::IntoCategories;
pub use logger::EguiLogger;
pub use record::LogRecord;
pub use types::{LogLevel, TimeFormat, TimePrecision};
pub use utils::{deserialize_color32, serialize_color32};
