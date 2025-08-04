#![doc = include_str!("../README.md")]

mod categories;
mod logger;
mod record;
mod types;
mod ui;
mod utils;

// Re-export public API
pub use categories::IntoCategories;
pub use logger::EguiLogger;
pub use record::LogRecord;
pub use types::{LogLevel, TimeFormat, TimePrecision};
pub use utils::{deserialize_color32, serialize_color32};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt;
    use std::fmt::Formatter;

    // Example usage and category enum
    #[derive(Debug, Clone, Copy)]
    #[allow(dead_code)]
    pub enum LogCategory {
        Unknown,
        Dialogue,
        Combat,
        UI,
        Network,
        Save,
        Load,
        Audio,
        Rendering,
    }

    impl fmt::Display for LogCategory {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            match self {
                LogCategory::Unknown => write!(f, "Unknown"),
                LogCategory::Dialogue => write!(f, "Dialogue"),
                LogCategory::Combat => write!(f, "Combat"),
                LogCategory::UI => write!(f, "UI"),
                LogCategory::Network => write!(f, "Network"),
                LogCategory::Save => write!(f, "Save"),
                LogCategory::Load => write!(f, "Load"),
                LogCategory::Audio => write!(f, "Audio"),
                LogCategory::Rendering => write!(f, "Rendering"),
            }
        }
    }

    #[test]
    fn test_basic_logging() {
        let mut logger = EguiLogger::new();
        logger.log(LogLevel::Info, vec![LogCategory::Dialogue], "Test message");

        assert_eq!(logger.total_records(), 1);
        let records = logger.filtered_records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].level, LogLevel::Info);
        assert_eq!(records[0].message, "Test message");
    }

    #[test]
    fn test_level_filtering() {
        let mut logger = EguiLogger::new();
        logger.log(LogLevel::Debug, vec![LogCategory::Unknown], "Debug msg");
        logger.log(LogLevel::Info, vec![LogCategory::Unknown], "Info msg");
        logger.log(LogLevel::Warn, vec![LogCategory::Unknown], "Warn msg");
        logger.log(LogLevel::Error, vec![LogCategory::Unknown], "Error msg");

        // Set max level to Warn - should show Warn and Error only
        logger.min_display_level = LogLevel::Warn;
        let visible = logger.filtered_records();
        assert_eq!(visible.len(), 2);
        assert!(visible.iter().all(|r| r.level >= LogLevel::Warn));
    }

    #[test]
    fn test_category_filtering() {
        let mut logger = EguiLogger::new();
        logger.log(LogLevel::Info, vec![LogCategory::Dialogue], "Dialogue msg");
        logger.log(LogLevel::Info, vec![LogCategory::Combat], "Combat msg");

        // Enable only Dialogue category
        logger.show_category(&LogCategory::Dialogue);
        logger.hide_category(&LogCategory::Combat);
        let visible = logger.filtered_records();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].categories[0], "Dialogue");
    }

    #[test]
    fn test_search_filtering() {
        let mut logger = EguiLogger::new();
        logger.log(LogLevel::Info, vec![LogCategory::Unknown], "Hello world");
        logger.log(LogLevel::Info, vec![LogCategory::Unknown], "Goodbye world");

        logger.search_term = "Hello".to_string();
        let visible = logger.filtered_records();
        assert_eq!(visible.len(), 1);
        assert!(visible[0].message.contains("Hello"));
    }

    #[test]
    fn test_message_truncation() {
        let mut logger = EguiLogger::new();
        logger.max_message_length = 10;

        logger.log(
            LogLevel::Info,
            vec![LogCategory::Unknown],
            "This is a very long message that should be truncated",
        );

        let records = logger.filtered_records();
        assert_eq!(records.len(), 1);
        assert!(records[0].message.len() <= 10);
        assert!(records[0].message.ends_with("..."));
    }
}
// TODO: Add a lot more tests.