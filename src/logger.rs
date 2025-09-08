use chrono::Local;
use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

use crate::categories::IntoCategories;
use crate::record::LogRecord;
use crate::types::{LogLevel, TimeFormat, TimePrecision};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EguiLogger {
    // TODO: Switch to a BinaryHeap. This will be way more efficient when iterating over all records
    //   since it will have them sorted by timestamp for free. It should also be able to do a fixed max size? Try using itertools?
    records: HashMap<LogLevel, VecDeque<LogRecord>>,

    /// Whether the entire settings bar should be shown. This option is not really meant for end
    /// users. It's for a developer to disable if they only want to display text.
    pub show_settings: bool,

    // Count of each category. Kept up to date as new records are added and old are removed.
    // Reports with multiple categories are counted once per category.
    // This is necessary to keep track of which categories exist, and which have been seen in the
    // past.
    category_counts: HashMap<String, u32>,

    /// Minimum log level to display (e.g. Info will display Info,Warn,Error but not Debug)
    pub min_display_level: LogLevel,
    /// Categories that should be hidden.
    /// New categories are shown by default. Note that categories may be saved here which
    /// aren't actually in the logger! This can happen if logs with those categories appear
    /// but are then either clear()'d or get pushed out of the buffer. In these cases, it's
    /// necessary to save categories here in order to save user preferences if case the logs
    /// show up again.
    hidden_categories: HashSet<String>,

    /// How to format timestamps
    pub time_format: TimeFormat,
    pub time_precision: TimePrecision,

    /// Whether to show a categories in the text.
    pub show_categories: bool,
    /// Whether to show log level in the text.
    pub show_level: bool,
    // Whether the search box is visible.
    pub show_search: bool,
    /// A trigger for the UI to focus on the search area. If true, the next rendered frame
    /// will set focus, and set this value to false.
    /// For example, if a user clicks the search button, it should immediately focus on the
    /// search input. That change is done through this trigger.
    #[serde(skip)]
    should_focus_search: bool,

    /// Maximum length per message (longer messages get truncated)
    /// Note that reducing this value won't truncate existing logs,
    /// increasing the value will not recover the truncated portion of logs.
    pub max_message_length: usize,
    /// Maximum number of records to keep per log level.
    /// Log levels are separated so that (for example) a bunch of Debug logs don't hide Error level
    /// logs. Note that in this example, the display might look odd at an older timestamp.
    /// There might appear to be many error logs and no debug logs, because the older debug logs
    /// have been removed. This is a natural consequence of this approach and is probably ok.
    /// TODO: Call this out in documentation, examples, and tests. Or find an alternative that's
    ///   more obvious to the user.
    /// TODO: Consider removing records after a time delay (a few hours? one game session?).
    pub max_records_per_level: usize,
    /// Current search term for filtering.
    pub search_term: String,
    // Storing this regex isn't important since it's generated from the search_term.
    // It only exists here to prevent the regex from being re-calculated on every frame.
    #[serde(skip)]
    search_regex: Option<Regex>,
    // Whether regex based searching is enabled.
    pub search_with_regex: bool,
    /// Whether search should be case sensitive. This also applies to regex search.
    pub search_with_case_sensitive: bool,

    // Fields related to the text box and user input.

    // Whether to show the entire text input section.
    pub show_input_area: bool,
    /// A trigger for the UI to focus on the text input area. If true, the next rendered frame
    /// will set focus to the input area, and set this value to false.
    #[serde(skip)]
    pub should_focus_input: bool,
    // The current user input.
    input_text: String,
    // When input_text is empty, this text is displayed to indicate where the input box is.
    pub input_hint: String,
    // A message to prepend to all user inputs.
    pub input_text_prefix: String,
    // Categories to apply to LogRecords triggered by user input.
    input_categories: Vec<String>,
    // The log level to apply to LogRecords triggered by user input.
    pub input_level: LogLevel,
}

impl Default for EguiLogger {
    fn default() -> Self {
        Self::new()
    }
}

fn default_records() -> HashMap<LogLevel, VecDeque<LogRecord>> {
    let mut records = HashMap::new();
    records.insert(LogLevel::Error, VecDeque::new());
    records.insert(LogLevel::Warn, VecDeque::new());
    records.insert(LogLevel::Info, VecDeque::new());
    records.insert(LogLevel::Debug, VecDeque::new());
    records
}

impl EguiLogger {
    pub fn new() -> Self {
        Self {
            records: default_records(),
            show_settings: true,
            category_counts: Default::default(),
            min_display_level: LogLevel::Debug,
            hidden_categories: HashSet::new(),
            time_format: TimeFormat::LocalTime,
            time_precision: TimePrecision::Seconds,
            show_categories: true,
            show_level: true,
            show_search: false,
            should_focus_search: false,
            max_message_length: 2000,
            max_records_per_level: 2000,
            search_term: String::new(),
            search_regex: None,
            search_with_regex: false,
            search_with_case_sensitive: false,
            show_input_area: true,
            should_focus_input: false,
            input_hint: "Type a message and press Enter...".to_string(),
            input_text: String::new(),
            input_text_prefix: String::new(),
            input_categories: vec!["Input".parse().unwrap()],
            input_level: LogLevel::Info,
        }
    }
    pub fn show(&mut self, ui: &mut egui::Ui) {
        crate::ui::render_logger_ui(self, ui);
    }

    pub fn log_error<C: IntoCategories, M: std::fmt::Display>(&mut self, categories: C, message: M) {
        self.log(LogLevel::Error, categories, message);
    }

    pub fn log_warn<C: IntoCategories, M: std::fmt::Display>(&mut self, categories: C, message: M) {
        self.log(LogLevel::Warn, categories, message);
    }

    pub fn log_info<C: IntoCategories, M: std::fmt::Display>(&mut self, categories: C, message: M) {
        self.log(LogLevel::Info, categories, message);
    }

    pub fn log_debug<C: IntoCategories, M: std::fmt::Display>(&mut self, categories: C, message: M) {
        self.log(LogLevel::Debug, categories, message);
    }

    /// Log a message with the given level and category
    pub fn log<C: IntoCategories, M: std::fmt::Display>(&mut self, level: LogLevel, categories: C, message: M) {
        let mut record = Self::get_log_record(level, categories, message);
        self.clean_record(&mut record);
        self.log_record(record);
    }

    /// Modifies the provided LogRecord to conform to the logger.
    fn clean_record(&self, record: &mut LogRecord) {
        if record.message.len() > self.max_message_length {
            record.message = format!("{}...", &record.message[..self.max_message_length.saturating_sub(3)]);
        }
    }

    /// Get a log record. This is the same LogRecord created by calling `log`.
    /// The record can be pushed into the chat using [`Self::log_record`].
    pub fn get_log_record<C: IntoCategories, M: std::fmt::Display>(
        level: LogLevel,
        categories: C,
        message: M,
    ) -> LogRecord {
        let category_strs = categories.into_categories();

        // Convert message to string without requiring &format!
        let message_str = message.to_string();
        let cleaned_message: String = message_str.chars().filter(|c| !c.eq(&'\n')).collect();

        LogRecord {
            timestamp: Local::now(),
            level,
            categories: category_strs,
            message: cleaned_message,
        }
    }

    /// Adds a LogRecord to the logs. The provided timestamp is used, so it will show up above existing messages if messages are provided out of order.
    pub fn log_record(&mut self, log_record: LogRecord) {
        log_record.categories.iter().for_each(|category| {
            self.category_counts
                .entry(category.to_string())
                .and_modify(|count| *count += 1)
                .or_insert(1);
        });

        self.records.get_mut(&log_record.level).unwrap().push_back(log_record);

        self.enforce_limits();
    }

    /// Enforce the maximum record limits for a single log level.
    fn enforce_limit(&mut self, level: &LogLevel) {
        let records = self.records.get_mut(level).unwrap();
        while records.len() > self.max_records_per_level {
            if let Some(r) = records.pop_front() {
                r.categories.iter().for_each(|category| {
                    self.category_counts
                        .entry(category.to_string())
                        .and_modify(|count| *count -= 1);
                })
            }
        }
    }

    /// Enforce the maximum record limits per level
    fn enforce_limits(&mut self) {
        self.enforce_limit(&LogLevel::Error);
        self.enforce_limit(&LogLevel::Warn);
        self.enforce_limit(&LogLevel::Info);
        self.enforce_limit(&LogLevel::Debug);
    }

    // Categories to apply to LogRecords triggered by user input.
    pub fn set_input_categories<T: ToString>(&mut self, categories: Vec<T>) {
        self.input_categories = categories.into_iter().map(|c| c.to_string()).collect();
    }

    /// Clear all log records
    pub fn clear(&mut self) {
        self.records.iter_mut().for_each(|(_, r)| r.clear());
        self.category_counts.clear();
    }

    /// Get all records that match current filters
    pub fn filtered_records(&self) -> Vec<&LogRecord> {
        self.records
            .values()
            .flatten()
            .filter(|record| self.matches_filters(record))
            .collect()
    }

    /// Check if a record matches current filters
    pub(crate) fn matches_filters(&self, record: &LogRecord) -> bool {
        // Level filtering (show this level and more severe)
        if record.level < self.min_display_level {
            return false;
        }

        // Category filtering
        if !self.hidden_categories.is_empty() {
            // TODO: Should it be this way, or based on all of the categories being hidden? What's more intuitive?
            // If any of a LogRecord's categories are marked as hidden, then hide the whole thing.
            if record.categories.iter().any(|cat| self.hidden_categories.contains(cat)) {
                return false;
            }
        }

        // Search filtering
        if !self.search_term.is_empty() {
            let formatted = self.format_record_text(record);
            let matches = if self.search_with_regex {
                // Note that the regex itself is generated to be case sensitive or not, so
                // that the regex + case check doesn't need to happen here.
                if self.search_regex.is_none() {
                    true
                } else {
                    self.search_regex.as_ref().unwrap().is_match(&formatted)
                }
            } else if self.search_with_case_sensitive {
                formatted.contains(&self.search_term)
            } else {
                formatted.to_lowercase().contains(&self.search_term.to_lowercase())
            };
            if !matches {
                return false;
            }
        }

        true
    }

    /// Get just the formatted text content without colors for search filtering
    pub(crate) fn format_record_text(&self, record: &LogRecord) -> String {
        let level_str = if self.show_level {
            format!("[{:}] ", record.level.as_str())
        } else {
            String::new()
        };
        let category_str = if self.show_categories {
            format!("[{:}] ", record.categories.join(","),)
        } else {
            String::new()
        };

        let time_str = format!(
            "{: >width$}",
            self.format_time(record.timestamp),
            width = self.get_time_format_padding()
        );

        format!("{}{}{}{}", time_str, level_str, category_str, record.message)
    }

    /// Get all unique categories that have been logged
    pub fn get_all_categories(&self) -> HashSet<String> {
        self.category_counts.keys().cloned().collect()
    }

    /// Get total number of records
    pub fn total_records(&self) -> usize {
        self.records.values().map(|r| r.len()).sum()
    }

    pub(crate) fn get_time_format_padding(&self) -> usize {
        // Format a time and use that to determine the padding to provide for all other rows.
        // TODO: Does this work? It feels like using a fixed timestamp would be better than
        //  non-determinism. Maybe I can get this value at compile-time, or find it myself
        //  and hard-code it in.
        self.format_time(Local::now()).len()
    }

    pub fn show_category<T: ToString>(&mut self, category: T) {
        self.hidden_categories.remove(&category.to_string());
    }

    pub fn hide_category<T: ToString>(&mut self, category: T) {
        self.hidden_categories.insert(category.to_string());
    }

    pub(crate) fn format_time(&self, time: chrono::DateTime<chrono::Local>) -> String {
        let time = match (self.time_format, self.time_precision) {
            (TimeFormat::Utc, TimePrecision::Seconds) => {
                time.to_utc().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
            }
            (TimeFormat::Utc, TimePrecision::Milliseconds) => {
                time.to_utc().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
            }
            (TimeFormat::LocalTime, TimePrecision::Seconds) => time.format("%T").to_string(),
            (TimeFormat::LocalTime, TimePrecision::Milliseconds) => time.format("%T%.3f").to_string(),
            (TimeFormat::Hide, _) => String::new(),
        };
        if self.time_format == TimeFormat::Hide {
            time
        } else {
            time + " "
        }
    }

    // Internal state accessors for UI
    pub(crate) fn records(&self) -> &HashMap<LogLevel, VecDeque<LogRecord>> {
        &self.records
    }

    pub(crate) fn category_counts(&self) -> &HashMap<String, u32> {
        &self.category_counts
    }

    pub(crate) fn hidden_categories(&self) -> &HashSet<String> {
        &self.hidden_categories
    }

    pub(crate) fn should_focus_search(&self) -> bool {
        self.should_focus_search
    }

    pub(crate) fn set_should_focus_search(&mut self, value: bool) {
        self.should_focus_search = value;
    }

    pub(crate) fn input_text(&self) -> &str {
        &self.input_text
    }

    pub(crate) fn input_text_mut(&mut self) -> &mut String {
        &mut self.input_text
    }

    pub(crate) fn take_input_text(&mut self) -> String {
        std::mem::take(&mut self.input_text)
    }

    pub(crate) fn input_categories(&self) -> &[String] {
        &self.input_categories
    }

    pub(crate) fn update_search_regex(&mut self) {
        if self.search_with_regex {
            self.search_regex = RegexBuilder::new(&self.search_term)
                .case_insensitive(!self.search_with_case_sensitive)
                .build()
                .ok();
        }
    }

    pub(crate) fn hidden_categories_mut(&mut self) -> &mut HashSet<String> {
        &mut self.hidden_categories
    }
}
