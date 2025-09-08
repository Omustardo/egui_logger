#[cfg(test)]
mod tests {
    use crate::{EguiLogger, LogLevel, TimeFormat, TimePrecision};
    use std::fmt;
    use std::fmt::Formatter;

    // Note: These tests don't actually create egui UI since that would require
    // a full egui context setup. Instead, they test the underlying state changes
    // that would occur from UI interactions.

    #[test]
    fn test_show_categories_toggle() {
        let mut logger = EguiLogger::new();

        // Initially show_categories should be true
        assert!(logger.show_categories);

        // Add a log record to test formatting
        logger.log(LogLevel::Info, vec![LogCategory::Dialogue], "Test message");

        // Get formatted text with categories shown
        assert_eq!(logger.filtered_records().len(), 1);
        let first_record = &logger.filtered_records()[0].clone();

        let formatted_text_with_cats = logger.format_record_text(first_record);
        assert!(formatted_text_with_cats.contains("[Dialogue]"));

        // Toggle show_categories
        logger.show_categories = false;

        // Get formatted text with categories hidden - need to get records again after state change
        let formatted_text = logger.format_record_text(first_record);

        // When show_categories is false, categories should not appear in formatted text
        assert!(!formatted_text.contains("[Dialogue]"));
        assert!(formatted_text.contains("Test message"));

        // Toggle back to show categories
        logger.show_categories = true;
        let formatted_text_with_cats_again = logger.format_record_text(first_record);
        assert!(formatted_text_with_cats_again.contains("[Dialogue]"));
    }

    #[test]
    fn test_show_level_toggle() {
        let mut logger = EguiLogger::new();

        // Initially show_level should be true
        assert!(logger.show_level);

        // Add a log record
        logger.log(LogLevel::Error, vec![LogCategory::Unknown], "Error message");
        assert_eq!(logger.filtered_records().len(), 1);
        let first_record = &logger.filtered_records()[0].clone();

        // With show_level true, level should appear in formatted text
        let formatted_with_level = logger.format_record_text(first_record);
        assert!(formatted_with_level.contains("[ERROR]"));

        // Toggle show_level off
        logger.show_level = false;
        let formatted_without_level = logger.format_record_text(first_record);
        assert!(!formatted_without_level.contains("[ERROR]"));
        assert!(formatted_without_level.contains("Error message"));
    }

    #[test]
    fn test_search_functionality_toggle() {
        let mut logger = EguiLogger::new();

        // Initially search should be hidden
        assert!(!logger.show_search);

        // Toggle search visibility
        logger.show_search = true;
        assert!(logger.show_search);

        // Test search focus trigger
        logger.set_should_focus_search(true);
        assert!(logger.should_focus_search());

        // Simulate focus being set (would happen in UI)
        logger.set_should_focus_search(false);
        assert!(!logger.should_focus_search());
    }

    #[test]
    fn test_input_area_toggle() {
        let mut logger = EguiLogger::new();

        // Initially input area should be shown
        assert!(logger.show_input_area);

        // Toggle input area off
        logger.show_input_area = false;
        assert!(!logger.show_input_area);

        // Toggle back on
        logger.show_input_area = true;
        assert!(logger.show_input_area);
    }

    #[test]
    fn test_log_level_filtering_interaction() {
        let mut logger = EguiLogger::new();

        // Add records of different levels
        logger.log(LogLevel::Debug, vec![LogCategory::Unknown], "Debug msg");
        logger.log(LogLevel::Info, vec![LogCategory::Unknown], "Info msg");
        logger.log(LogLevel::Warn, vec![LogCategory::Unknown], "Warn msg");
        logger.log(LogLevel::Error, vec![LogCategory::Unknown], "Error msg");

        assert_eq!(logger.total_records(), 4);

        // Initially all should be visible (min_display_level is Debug by default)
        assert_eq!(logger.filtered_records().len(), 4);

        // Simulate clicking on "Info" level filter
        logger.min_display_level = LogLevel::Info;
        let visible_records = logger.filtered_records();
        assert_eq!(visible_records.len(), 3); // Info, Warn, Error
        assert!(visible_records.iter().all(|r| r.level >= LogLevel::Info));

        // Simulate clicking on "Error" level filter
        logger.min_display_level = LogLevel::Error;
        let visible_records = logger.filtered_records();
        assert_eq!(visible_records.len(), 1); // Only Error
        assert!(visible_records.iter().all(|r| r.level >= LogLevel::Error));
    }

    #[test]
    fn test_category_filtering_interaction() {
        let mut logger = EguiLogger::new();

        // Add records with different categories
        logger.log(LogLevel::Info, vec![LogCategory::Dialogue], "Dialogue msg");
        logger.log(LogLevel::Info, vec![LogCategory::Combat], "Combat msg");
        logger.log(LogLevel::Info, vec![LogCategory::UI], "UI msg");

        assert_eq!(logger.total_records(), 3);
        assert_eq!(logger.filtered_records().len(), 3);

        // Simulate hiding Combat category (like clicking in Categories menu)
        logger.hide_category(LogCategory::Combat);
        let visible_records = logger.filtered_records();
        assert_eq!(visible_records.len(), 2);
        assert!(
            !visible_records
                .iter()
                .any(|r| r.categories.contains(&"Combat".to_string()))
        );

        // Simulate hiding all categories except Dialogue
        logger.hide_category(LogCategory::UI);
        let visible_records = logger.filtered_records();
        assert_eq!(visible_records.len(), 1);
        assert_eq!(visible_records[0].categories[0], "Dialogue");

        // Simulate "Select All" in Categories menu
        logger.show_category(LogCategory::Combat);
        logger.show_category(LogCategory::UI);
        let visible_records = logger.filtered_records();
        assert_eq!(visible_records.len(), 3);
    }

    #[test]
    fn test_search_interaction() {
        let mut logger = EguiLogger::new();

        // Add some test records
        logger.log(LogLevel::Info, vec![LogCategory::Unknown], "Hello world");
        logger.log(LogLevel::Info, vec![LogCategory::Unknown], "Goodbye world");
        logger.log(LogLevel::Info, vec![LogCategory::Unknown], "Testing search");

        assert_eq!(logger.filtered_records().len(), 3);

        // Simulate typing in search box
        logger.search_term = "Hello".to_string();
        let visible_records = logger.filtered_records();
        assert_eq!(visible_records.len(), 1);
        assert!(visible_records[0].message.contains("Hello"));

        // Test case sensitive search
        logger.search_term = "hello".to_string();
        logger.search_with_case_sensitive = true;
        let visible_records = logger.filtered_records();
        assert_eq!(visible_records.len(), 0); // Should not match due to case sensitivity

        // Turn off case sensitivity
        logger.search_with_case_sensitive = false;
        let visible_records = logger.filtered_records();
        assert_eq!(visible_records.len(), 1); // Should match again

        // Test regex search
        logger.search_term = "w.rld".to_string();
        logger.search_with_regex = true;
        logger.update_search_regex();
        let visible_records = logger.filtered_records();
        assert_eq!(visible_records.len(), 2); // Should match both "Hello world" and "Goodbye world"
    }

    #[test]
    fn test_time_format_interaction() {
        let mut logger = EguiLogger::new();

        // Add a test record
        logger.log(LogLevel::Info, vec![LogCategory::Unknown], "Test message");
        assert_eq!(logger.filtered_records().len(), 1);
        let first_record = &logger.filtered_records()[0].clone();

        // Test different time formats
        logger.time_format = TimeFormat::LocalTime;
        let formatted_local = logger.format_record_text(first_record);
        assert!(formatted_local.len() > "Test message".len()); // Should have timestamp

        logger.time_format = TimeFormat::Hide;
        let formatted_hidden = logger.format_record_text(first_record);
        // When time is hidden, formatted text should be shorter
        assert!(formatted_hidden.len() < formatted_local.len());

        // Test precision changes
        logger.time_format = TimeFormat::LocalTime;
        logger.time_precision = TimePrecision::Milliseconds;
        let formatted_millis = logger.format_record_text(first_record);

        logger.time_precision = TimePrecision::Seconds;
        let formatted_seconds = logger.format_record_text(first_record);

        // Millisecond format should be longer than seconds format
        assert!(formatted_millis.len() > formatted_seconds.len());
    }

    #[test]
    fn test_input_text_interaction() {
        let mut logger = EguiLogger::new();

        // Test initial state
        assert!(logger.input_text().is_empty());
        assert_eq!(logger.input_categories(), &["Input"]);
        assert_eq!(logger.input_level, LogLevel::Info);

        // Simulate typing in input field
        *logger.input_text_mut() = "User typed message".to_string();
        assert_eq!(logger.input_text(), "User typed message");

        // Simulate submitting input (like pressing Enter)
        let initial_count = logger.total_records();
        let submitted_text = logger.take_input_text();
        assert_eq!(submitted_text, "User typed message");
        assert!(logger.input_text().is_empty()); // Should be cleared after taking

        // Simulate the log that would be created from input
        logger.log(logger.input_level, logger.input_categories().to_vec(), &submitted_text);
        assert_eq!(logger.total_records(), initial_count + 1);

        let records = logger.filtered_records();
        let input_record = records.iter().find(|r| r.message == "User typed message").unwrap();
        assert_eq!(input_record.level, LogLevel::Info);
        assert_eq!(input_record.categories[0], "Input");
    }

    #[test]
    fn test_clear_interaction() {
        let mut logger = EguiLogger::new();

        // Add some records
        logger.log(LogLevel::Info, vec![LogCategory::Dialogue], "Message 1");
        logger.log(LogLevel::Error, vec![LogCategory::Combat], "Message 2");

        assert_eq!(logger.total_records(), 2);
        assert_eq!(logger.get_all_categories().len(), 2);

        // Simulate clicking Clear button
        logger.clear();

        assert_eq!(logger.total_records(), 0);
        assert_eq!(logger.get_all_categories().len(), 0);
        assert_eq!(logger.filtered_records().len(), 0);
    }

    #[test]
    fn test_input_focus_trigger() {
        let mut logger = EguiLogger::new();

        // Initially should not need focus
        assert!(!logger.should_focus_input);

        // Simulate triggering input focus (like clicking input area)
        logger.should_focus_input = true;
        assert!(logger.should_focus_input);

        // Simulate focus being applied (would happen in UI render)
        logger.should_focus_input = false;
        assert!(!logger.should_focus_input);
    }

    #[test]
    fn test_input_categories_modification() {
        let mut logger = EguiLogger::new();

        // Test initial input categories
        assert_eq!(logger.input_categories(), &["Input"]);

        // Simulate changing input categories (like through settings)
        logger.set_input_categories(vec!["UserInput", "Console"]);
        assert_eq!(logger.input_categories(), &["UserInput", "Console"]);

        // Test that logs created with input use these categories
        logger.log(logger.input_level, logger.input_categories().to_vec(), "Test input");
        let records = logger.filtered_records();
        let input_record = records.iter().find(|r| r.message == "Test input").unwrap();
        assert_eq!(input_record.categories, vec!["UserInput", "Console"]);
    }

    #[test]
    fn test_multiple_filter_interactions() {
        let mut logger = EguiLogger::new();

        // Add diverse test data
        logger.log(LogLevel::Debug, vec![LogCategory::Dialogue], "Debug dialogue");
        logger.log(LogLevel::Info, vec![LogCategory::Dialogue], "Info dialogue");
        logger.log(LogLevel::Info, vec![LogCategory::Combat], "Info combat");
        logger.log(LogLevel::Error, vec![LogCategory::Combat], "Error combat");

        assert_eq!(logger.total_records(), 4);
        assert_eq!(logger.filtered_records().len(), 4);

        // Apply level filter (Info and above)
        logger.min_display_level = LogLevel::Info;
        assert_eq!(logger.filtered_records().len(), 3); // Excludes Debug

        // Add category filter (hide Combat)
        logger.hide_category(LogCategory::Combat);
        assert_eq!(logger.filtered_records().len(), 1); // Only Info dialogue remains

        // Add search filter
        logger.search_term = "dialogue".to_string();
        assert_eq!(logger.filtered_records().len(), 1); // Still matches

        logger.search_term = "combat".to_string();
        assert_eq!(logger.filtered_records().len(), 0); // Combat is hidden by category filter

        // Remove category filter but keep search
        logger.show_category(LogCategory::Combat);
        assert_eq!(logger.filtered_records().len(), 2); // Now finds both "Info combat" and "Error combat"

        // Clear search
        logger.search_term.clear();
        assert_eq!(logger.filtered_records().len(), 3); // Back to level filter only
    }

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
